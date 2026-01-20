# Issue轮询方案设计 (Webhook备选方案)

**版本**: v0.1.0  
**日期**: 2026-01-20  
**状态**: 设计中

## 1. 背景和动机

### 1.1 为什么需要轮询方案?

**Webhook的局限性**:
- ❌ 需要公网可访问的URL
- ❌ 需要配置防火墙/反向代理
- ❌ 本地开发环境难以测试
- ❌ 某些Git平台可能不支持Webhook
- ❌ Webhook可能因网络问题丢失事件

**轮询方案的优势**:
- ✅ 无需公网URL
- ✅ 适合本地开发和测试
- ✅ 不依赖Git平台的Webhook功能
- ✅ 可以补偿Webhook丢失的事件
- ✅ 实现简单，易于调试

### 1.2 使用场景

1. **本地开发**: 开发者在本地测试时无需配置Webhook
2. **内网部署**: 企业内网环境无法暴露公网URL
3. **Webhook备份**: 作为Webhook的补充，防止事件丢失
4. **平台兼容**: 某些Git平台不支持Webhook时的备选方案

### 1.3 设计目标

- 🎯 定期轮询Git平台获取新Issue
- 🎯 避免重复创建Task
- 🎯 支持多Repository并发轮询
- 🎯 可配置轮询间隔
- 🎯 低资源消耗
- 🎯 与Webhook方案共存

## 2. 现有基础设施

### 2.1 GitProvider API (已实现)

```rust
#[async_trait]
pub trait GitProvider {
    /// 列出Issues (支持过滤)
    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError>;
    
    /// 获取单个Issue
    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitIssue, GitProviderError>;
}

/// Issue过滤器
pub struct IssueFilter {
    pub state: Option<IssueState>,      // Open/Closed
    pub labels: Option<Vec<String>>,    // 标签过滤
    pub assignee: Option<String>,       // 指派人过滤
}

/// Issue数据模型
pub struct GitIssue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 2.2 后台服务框架 (已实现)

```rust
#[async_trait]
pub trait BackgroundService: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self) -> Result<()>;
    fn interval(&self) -> Duration;
}
```

**现有后台服务**:
- `WebhookRetryService` - Webhook重试服务 (每5分钟)
- `LogCleanupService` - 日志清理服务 (每天)
- `RepositorySyncService` - 仓库同步服务 (每小时)

## 3. 核心设计

### 3.1 轮询策略

#### 策略1: 基于updated_at的增量轮询 (推荐)

**原理**: 只获取自上次轮询后更新的Issue

```rust
// 伪代码
last_poll_time = get_last_poll_time(repository_id);
issues = git_provider.list_issues(owner, repo, IssueFilter {
    state: Some(IssueState::Open),
    labels: Some(vec!["vibe-auto".to_string()]),
    ..Default::default()
});

// 过滤出新Issue或更新的Issue
new_issues = issues.filter(|issue| {
    issue.created_at > last_poll_time || issue.updated_at > last_poll_time
});

// 处理新Issue
for issue in new_issues {
    if issue.created_at > last_poll_time {
        // 新创建的Issue → 创建Task
        create_task_from_issue(issue);
    } else {
        // 已存在的Issue被更新 → 检查是否需要处理
        handle_issue_update(issue);
    }
}

// 更新轮询时间戳
update_last_poll_time(repository_id, now);
```

**优点**:
- ✅ 只处理变化的Issue，效率高
- ✅ 减少API调用次数
- ✅ 避免重复处理

**缺点**:
- ❌ 需要存储last_poll_time
- ❌ 依赖Git平台的updated_at字段准确性

#### 策略2: 全量轮询 + 去重

**原理**: 每次获取所有Open状态的Issue，通过数据库去重

```rust
// 获取所有Open的Issue
issues = git_provider.list_issues(owner, repo, IssueFilter {
    state: Some(IssueState::Open),
    labels: Some(vec!["vibe-auto".to_string()]),
    ..Default::default()
});

// 检查每个Issue是否已有Task
for issue in issues {
    let existing_task = find_task_by_issue(workspace_id, issue.number);
    
    if existing_task.is_none() {
        // 没有对应的Task → 创建
        create_task_from_issue(issue);
    }
}
```

**优点**:
- ✅ 实现简单
- ✅ 不需要存储状态
- ✅ 可以补偿遗漏的Issue

**缺点**:
- ❌ 每次都要查询所有Issue
- ❌ 数据库查询次数多
- ❌ API调用开销大

### 3.2 数据库Schema扩展

需要在`repositories`表或新建`polling_state`表存储轮询状态。

#### 方案A: 扩展repositories表 (推荐)

```sql
ALTER TABLE repositories ADD COLUMN last_issue_poll_at TIMESTAMP;
ALTER TABLE repositories ADD COLUMN polling_enabled BOOLEAN DEFAULT FALSE;
ALTER TABLE repositories ADD COLUMN polling_interval_seconds INTEGER DEFAULT 300;
```

**字段说明**:
- `last_issue_poll_at`: 上次轮询时间
- `polling_enabled`: 是否启用轮询 (可与Webhook共存)
- `polling_interval_seconds`: 轮询间隔 (秒)

#### 方案B: 新建polling_state表

```sql
CREATE TABLE polling_state (
    id INTEGER PRIMARY KEY,
    repository_id INTEGER NOT NULL UNIQUE,
    last_poll_at TIMESTAMP NOT NULL,
    last_issue_number INTEGER,
    enabled BOOLEAN DEFAULT TRUE,
    interval_seconds INTEGER DEFAULT 300,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
```

**优点**: 解耦，不污染repositories表  
**缺点**: 增加表数量，需要JOIN查询

### 3.3 去重机制

**问题**: 如何避免为同一个Issue重复创建Task?

**解决方案**: 数据库唯一约束 + 查询检查

#### 方案1: 添加唯一索引 (推荐)

```sql
-- 在tasks表添加唯一约束
CREATE UNIQUE INDEX idx_tasks_workspace_issue 
ON tasks(workspace_id, issue_number) 
WHERE deleted_at IS NULL;
```

**优点**:
- ✅ 数据库层面保证唯一性
- ✅ 并发安全
- ✅ 性能好

**实现**:
```rust
async fn create_task_from_issue(
    workspace_id: i32,
    issue: &GitIssue,
    db: &DatabaseConnection,
) -> Result<task::Model> {
    let task = task::ActiveModel {
        workspace_id: Set(workspace_id),
        issue_number: Set(issue.number as i32),
        issue_title: Set(issue.title.clone()),
        issue_body: Set(issue.body.clone()),
        task_status: Set("Pending".to_string()),
        priority: Set("Medium".to_string()),
        ..Default::default()
    };
    
    // 尝试插入，如果违反唯一约束则忽略
    match Task::insert(task).exec_with_returning(db).await {
        Ok(task) => {
            tracing::info!(
                workspace_id = workspace_id,
                issue_number = issue.number,
                "Created task from issue"
            );
            Ok(task)
        }
        Err(DbErr::RecordNotInserted) => {
            // 已存在，跳过
            tracing::debug!(
                workspace_id = workspace_id,
                issue_number = issue.number,
                "Task already exists, skipping"
            );
            Err(VibeRepoError::Conflict("Task already exists".to_string()))
        }
        Err(e) => Err(VibeRepoError::Database(e)),
    }
}
```

#### 方案2: 查询检查

```rust
async fn ensure_task_exists(
    workspace_id: i32,
    issue: &GitIssue,
    db: &DatabaseConnection,
) -> Result<task::Model> {
    // 先查询是否存在
    let existing = Task::find()
        .filter(task::Column::WorkspaceId.eq(workspace_id))
        .filter(task::Column::IssueNumber.eq(issue.number as i32))
        .filter(task::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    
    if let Some(task) = existing {
        return Ok(task);
    }
    
    // 不存在则创建
    create_task_from_issue(workspace_id, issue, db).await
}
```

**缺点**: 存在竞态条件 (两个请求同时检查都不存在，然后都尝试创建)

### 3.4 Issue过滤规则

**问题**: 不是所有Issue都应该创建Task

**过滤条件**:
1. **状态过滤**: 只处理Open状态的Issue
2. **标签过滤**: 只处理带特定标签的Issue (如`vibe-auto`)
3. **@mention过滤**: 只处理提到bot的Issue
4. **时间过滤**: 只处理最近创建/更新的Issue

```rust
fn should_create_task(issue: &GitIssue, config: &PollingConfig) -> bool {
    // 1. 必须是Open状态
    if issue.state != IssueState::Open {
        return false;
    }
    
    // 2. 检查标签 (如果配置了required_labels)
    if let Some(required_labels) = &config.required_labels {
        if !required_labels.iter().any(|label| issue.labels.contains(label)) {
            return false;
        }
    }
    
    // 3. 检查@mention (如果配置了bot_username)
    if let Some(bot_username) = &config.bot_username {
        if let Some(body) = &issue.body {
            if !body.contains(&format!("@{}", bot_username)) {
                return false;
            }
        } else {
            return false;
        }
    }
    
    // 4. 检查时间窗口 (避免处理太旧的Issue)
    if let Some(max_age_days) = config.max_issue_age_days {
        let age = Utc::now() - issue.created_at;
        if age.num_days() > max_age_days {
            return false;
        }
    }
    
    true
}
```

## 4. 实现方案

### 4.1 IssuePollingService

```rust
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set};
use std::time::Duration;

use crate::entities::prelude::*;
use crate::entities::{repository, workspace};
use crate::error::Result;
use crate::git_provider::{GitClientFactory, IssueFilter, IssueState};
use crate::services::{BackgroundService, TaskService};

pub struct IssuePollingService {
    db: DatabaseConnection,
    config: PollingConfig,
}

#[derive(Clone)]
pub struct PollingConfig {
    /// 轮询间隔 (秒)
    pub interval_seconds: u64,
    /// 是否启用轮询
    pub enabled: bool,
    /// 必需的标签 (可选)
    pub required_labels: Option<Vec<String>>,
    /// Bot用户名 (用于@mention检查)
    pub bot_username: Option<String>,
    /// 最大Issue年龄 (天)
    pub max_issue_age_days: Option<i64>,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 300, // 5分钟
            enabled: true,
            required_labels: Some(vec!["vibe-auto".to_string()]),
            bot_username: Some("gitautodev-bot".to_string()),
            max_issue_age_days: Some(30), // 只处理30天内的Issue
        }
    }
}

impl IssuePollingService {
    pub fn new(db: DatabaseConnection, config: PollingConfig) -> Self {
        Self { db, config }
    }
    
    /// 轮询所有启用的Repository
    async fn poll_all_repositories(&self) -> Result<()> {
        // 获取所有启用轮询的Repository
        let repositories = Repository::find()
            .filter(repository::Column::PollingEnabled.eq(true))
            .filter(repository::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        
        tracing::info!(
            count = repositories.len(),
            "Polling issues for repositories"
        );
        
        for repo in repositories {
            if let Err(e) = self.poll_repository(&repo).await {
                tracing::error!(
                    repository_id = repo.id,
                    repository_name = %repo.full_name,
                    error = %e,
                    "Failed to poll repository"
                );
            }
        }
        
        Ok(())
    }
    
    /// 轮询单个Repository
    async fn poll_repository(&self, repo: &repository::Model) -> Result<()> {
        tracing::debug!(
            repository_id = repo.id,
            repository_name = %repo.full_name,
            "Polling repository for issues"
        );
        
        // 1. 获取Provider和GitClient
        let provider = RepoProvider::find_by_id(repo.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound(
                format!("Provider {} not found", repo.provider_id)
            ))?;
        
        let client = GitClientFactory::from_provider(&provider)?;
        
        // 2. 解析owner/repo
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Validation(
                format!("Invalid repository name: {}", repo.full_name)
            ));
        }
        let (owner, repo_name) = (parts[0], parts[1]);
        
        // 3. 获取Workspace
        let workspace = Workspace::find()
            .filter(workspace::Column::RepositoryId.eq(repo.id))
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound(
                format!("Workspace not found for repository {}", repo.id)
            ))?;
        
        // 4. 构建Issue过滤器
        let filter = IssueFilter {
            state: Some(IssueState::Open),
            labels: self.config.required_labels.clone(),
            assignee: None,
        };
        
        // 5. 获取Issues
        let issues = client.list_issues(owner, repo_name, Some(filter)).await?;
        
        tracing::info!(
            repository_id = repo.id,
            issue_count = issues.len(),
            "Fetched issues from repository"
        );
        
        // 6. 过滤和处理Issues
        let last_poll_time = repo.last_issue_poll_at.unwrap_or_else(|| {
            // 如果没有上次轮询时间，使用30天前
            Utc::now() - chrono::Duration::days(30)
        });
        
        let mut new_task_count = 0;
        for issue in issues {
            // 只处理新创建的Issue
            if issue.created_at <= last_poll_time {
                continue;
            }
            
            // 应用过滤规则
            if !self.should_create_task(&issue) {
                tracing::debug!(
                    issue_number = issue.number,
                    "Issue filtered out"
                );
                continue;
            }
            
            // 创建Task
            match self.create_task_from_issue(workspace.id, &issue).await {
                Ok(_) => {
                    new_task_count += 1;
                    tracing::info!(
                        repository_id = repo.id,
                        issue_number = issue.number,
                        "Created task from issue"
                    );
                }
                Err(VibeRepoError::Conflict(_)) => {
                    // Task已存在，跳过
                    tracing::debug!(
                        issue_number = issue.number,
                        "Task already exists"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        issue_number = issue.number,
                        error = %e,
                        "Failed to create task"
                    );
                }
            }
        }
        
        // 7. 更新last_poll_time
        let mut repo_active: repository::ActiveModel = repo.clone().into();
        repo_active.last_issue_poll_at = Set(Some(Utc::now()));
        repo_active.update(&self.db).await?;
        
        tracing::info!(
            repository_id = repo.id,
            new_tasks = new_task_count,
            "Completed repository polling"
        );
        
        Ok(())
    }
    
    fn should_create_task(&self, issue: &GitIssue) -> bool {
        // 实现过滤逻辑 (见3.4节)
        // ...
        true
    }
    
    async fn create_task_from_issue(
        &self,
        workspace_id: i32,
        issue: &GitIssue,
    ) -> Result<task::Model> {
        // 实现Task创建逻辑 (见3.3节)
        // ...
        todo!()
    }
}

#[async_trait]
impl BackgroundService for IssuePollingService {
    fn name(&self) -> &str {
        "issue_polling"
    }
    
    async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            tracing::debug!("Issue polling is disabled");
            return Ok(());
        }
        
        self.poll_all_repositories().await
    }
    
    fn interval(&self) -> Duration {
        Duration::from_secs(self.config.interval_seconds)
    }
}
```

### 4.2 配置管理

#### 环境变量配置

```bash
# .env
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
ISSUE_POLLING_REQUIRED_LABELS=vibe-auto,auto-dev
ISSUE_POLLING_BOT_USERNAME=gitautodev-bot
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
```

#### AppConfig扩展

```rust
// backend/src/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // ... 现有字段
    
    #[serde(default)]
    pub issue_polling: IssuePollingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssuePollingConfig {
    #[serde(default = "default_polling_enabled")]
    pub enabled: bool,
    
    #[serde(default = "default_polling_interval")]
    pub interval_seconds: u64,
    
    pub required_labels: Option<Vec<String>>,
    pub bot_username: Option<String>,
    
    #[serde(default = "default_max_age")]
    pub max_issue_age_days: Option<i64>,
}

fn default_polling_enabled() -> bool { false }
fn default_polling_interval() -> u64 { 300 }
fn default_max_age() -> Option<i64> { Some(30) }

impl Default for IssuePollingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 300,
            required_labels: Some(vec!["vibe-auto".to_string()]),
            bot_username: Some("gitautodev-bot".to_string()),
            max_issue_age_days: Some(30),
        }
    }
}
```

### 4.3 API端点

提供API控制单个Repository的轮询配置。

```rust
// backend/src/api/repositories/handlers.rs

/// 启用/禁用Repository的Issue轮询
#[utoipa::path(
    patch,
    path = "/api/repositories/{id}/polling",
    request_body = UpdatePollingRequest,
    responses(
        (status = 200, description = "Polling config updated", body = RepositoryResponse),
        (status = 404, description = "Repository not found"),
    ),
    tag = "repositories"
)]
pub async fn update_repository_polling(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdatePollingRequest>,
) -> Result<Json<RepositoryResponse>> {
    let repo = Repository::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(
            format!("Repository {} not found", id)
        ))?;
    
    let mut repo_active: repository::ActiveModel = repo.into();
    repo_active.polling_enabled = Set(req.enabled);
    if let Some(interval) = req.interval_seconds {
        repo_active.polling_interval_seconds = Set(interval);
    }
    repo_active.updated_at = Set(Utc::now());
    
    let repo = repo_active.update(&state.db).await?;
    
    Ok(Json(repo.into()))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePollingRequest {
    pub enabled: bool,
    pub interval_seconds: Option<i32>,
}

/// 手动触发Repository的Issue轮询
#[utoipa::path(
    post,
    path = "/api/repositories/{id}/poll-issues",
    responses(
        (status = 200, description = "Polling triggered", body = PollIssuesResponse),
        (status = 404, description = "Repository not found"),
    ),
    tag = "repositories"
)]
pub async fn trigger_issue_polling(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<PollIssuesResponse>> {
    let repo = Repository::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(
            format!("Repository {} not found", id)
        ))?;
    
    // 创建轮询服务并执行
    let polling_service = IssuePollingService::new(
        state.db.clone(),
        state.config.issue_polling.clone().into(),
    );
    
    // 异步执行轮询
    let repo_clone = repo.clone();
    tokio::spawn(async move {
        if let Err(e) = polling_service.poll_repository(&repo_clone).await {
            tracing::error!(
                repository_id = repo_clone.id,
                error = %e,
                "Manual polling failed"
            );
        }
    });
    
    Ok(Json(PollIssuesResponse {
        success: true,
        message: "Polling triggered".to_string(),
    }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PollIssuesResponse {
    pub success: bool,
    pub message: String,
}
```

## 5. 性能优化

### 5.1 批量处理

**问题**: 大量Repository时，串行轮询效率低

**解决方案**: 并发轮询

```rust
async fn poll_all_repositories(&self) -> Result<()> {
    let repositories = Repository::find()
        .filter(repository::Column::PollingEnabled.eq(true))
        .filter(repository::Column::DeletedAt.is_null())
        .all(&self.db)
        .await?;
    
    // 使用futures::stream并发处理
    use futures::stream::{self, StreamExt};
    
    let results = stream::iter(repositories)
        .map(|repo| async move {
            self.poll_repository(&repo).await
        })
        .buffer_unordered(10) // 最多10个并发
        .collect::<Vec<_>>()
        .await;
    
    // 统计结果
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let error_count = results.len() - success_count;
    
    tracing::info!(
        total = results.len(),
        success = success_count,
        errors = error_count,
        "Completed polling all repositories"
    );
    
    Ok(())
}
```

### 5.2 智能轮询间隔

**问题**: 固定间隔可能不够灵活

**解决方案**: 根据Repository活跃度动态调整

```rust
fn calculate_polling_interval(repo: &repository::Model) -> Duration {
    // 基础间隔
    let base_interval = repo.polling_interval_seconds.unwrap_or(300);
    
    // 如果最近没有新Issue，延长间隔
    if let Some(last_poll) = repo.last_issue_poll_at {
        let since_last_poll = Utc::now() - last_poll;
        
        // 如果超过1小时没有新Issue，间隔翻倍 (最多1小时)
        if since_last_poll.num_hours() > 1 {
            return Duration::from_secs(
                std::cmp::min(base_interval * 2, 3600)
            );
        }
    }
    
    Duration::from_secs(base_interval)
}
```

### 5.3 缓存优化

**问题**: 频繁查询数据库

**解决方案**: 缓存Repository和Workspace映射

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct IssuePollingService {
    db: DatabaseConnection,
    config: PollingConfig,
    // 缓存: repository_id -> workspace_id
    workspace_cache: Arc<RwLock<HashMap<i32, i32>>>,
}

impl IssuePollingService {
    async fn get_workspace_id(&self, repo_id: i32) -> Result<i32> {
        // 先查缓存
        {
            let cache = self.workspace_cache.read().await;
            if let Some(workspace_id) = cache.get(&repo_id) {
                return Ok(*workspace_id);
            }
        }
        
        // 缓存未命中，查数据库
        let workspace = Workspace::find()
            .filter(workspace::Column::RepositoryId.eq(repo_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound(
                format!("Workspace not found for repository {}", repo_id)
            ))?;
        
        // 更新缓存
        {
            let mut cache = self.workspace_cache.write().await;
            cache.insert(repo_id, workspace.id);
        }
        
        Ok(workspace.id)
    }
}
```

### 5.4 API限流保护

**问题**: 频繁调用Git平台API可能触发限流

**解决方案**: 
1. 增加轮询间隔
2. 使用指数退避
3. 监控API配额

```rust
async fn poll_repository_with_retry(&self, repo: &repository::Model) -> Result<()> {
    let mut retry_count = 0;
    let max_retries = 3;
    
    loop {
        match self.poll_repository(repo).await {
            Ok(_) => return Ok(()),
            Err(VibeRepoError::GitProvider(GitProviderError::RateLimitExceeded)) => {
                if retry_count >= max_retries {
                    return Err(VibeRepoError::Internal(
                        "Rate limit exceeded after retries".to_string()
                    ));
                }
                
                // 指数退避: 2^retry_count 分钟
                let delay_secs = 2_u64.pow(retry_count) * 60;
                tracing::warn!(
                    repository_id = repo.id,
                    retry_count = retry_count,
                    delay_secs = delay_secs,
                    "Rate limited, retrying after delay"
                );
                
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                retry_count += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```




## 6. 与Webhook方案的协同

### 6.1 双模式运行

**设计原则**: 轮询和Webhook可以同时启用，互为补充

```rust
// Repository配置
pub struct Repository {
    // Webhook相关
    pub webhook_status: String,        // "active", "failed", "disabled"
    
    // Polling相关
    pub polling_enabled: bool,         // 是否启用轮询
    pub polling_interval_seconds: i32, // 轮询间隔
    pub last_issue_poll_at: Option<DateTimeUtc>, // 上次轮询时间
}
```

**运行模式**:

| Webhook | Polling | 说明 |
|---------|---------|------|
| ✅ Active | ❌ Disabled | 纯Webhook模式 (推荐) |
| ❌ Failed | ✅ Enabled | 纯轮询模式 (备选) |
| ✅ Active | ✅ Enabled | 双模式 (最可靠) |
| ❌ Disabled | ❌ Disabled | 不处理Issue |

### 6.2 去重保证

**问题**: Webhook和轮询可能同时创建Task

**解决方案**: 数据库唯一约束 (见3.3节)

```sql
CREATE UNIQUE INDEX idx_tasks_workspace_issue 
ON tasks(workspace_id, issue_number) 
WHERE deleted_at IS NULL;
```

**效果**:
- Webhook先创建 → 轮询尝试创建时失败 (Conflict)
- 轮询先创建 → Webhook尝试创建时失败 (Conflict)
- 两者同时创建 → 数据库保证只有一个成功

### 6.3 故障切换

**场景**: Webhook失败时自动启用轮询

```rust
// WebhookRetryService中
async fn handle_webhook_failure(&self, repo_id: i32) -> Result<()> {
    let repo = Repository::find_by_id(repo_id)
        .one(&self.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(
            format!("Repository {} not found", repo_id)
        ))?;
    
    // 如果Webhook失败次数超过阈值，自动启用轮询
    if repo.webhook_retry_count >= 5 && !repo.polling_enabled {
        tracing::warn!(
            repository_id = repo_id,
            "Webhook failed multiple times, enabling polling as fallback"
        );
        
        let mut repo_active: repository::ActiveModel = repo.into();
        repo_active.polling_enabled = Set(true);
        repo_active.update(&self.db).await?;
    }
    
    Ok(())
}
```

## 7. 实现计划

### Phase 1: 基础轮询功能 (2-3天)

**任务清单**:
1. [ ] 数据库Migration
   - 添加`polling_enabled`, `polling_interval_seconds`, `last_issue_poll_at`字段
   - 添加唯一索引`idx_tasks_workspace_issue_unique`
2. [ ] 实现`IssuePollingService`
   - `poll_all_repositories()`
   - `poll_repository()`
   - `should_create_task()`
   - `create_task_from_issue()`
3. [ ] 配置管理
   - 扩展`AppConfig`
   - 环境变量支持
4. [ ] 单元测试

**验收标准**:
- 轮询服务能正常运行
- 能从Git平台获取Issue
- 能创建Task (去重正常)

### Phase 2: API和配置 (1-2天)

**任务清单**:
1. [ ] Repository API扩展
   - `PATCH /api/repositories/:id/polling` - 更新轮询配置
   - `POST /api/repositories/:id/poll-issues` - 手动触发轮询
2. [ ] 响应模型
   - `UpdatePollingRequest`
   - `PollIssuesResponse`
3. [ ] OpenAPI文档
4. [ ] 集成测试

**验收标准**:
- API正常工作
- 可以通过API控制轮询
- 文档完整

### Phase 3: 性能优化 (1-2天)

**任务清单**:
1. [ ] 并发轮询
   - 使用`futures::stream`
   - 限制并发数
2. [ ] 缓存优化
   - Workspace映射缓存
3. [ ] API限流保护
   - 指数退避重试
4. [ ] 性能测试

**验收标准**:
- 100个Repository轮询 < 30秒
- API限流时能正确重试
- 缓存命中率 > 80%

### Phase 4: 与Webhook集成 (1天)

**任务清单**:
1. [ ] 双模式支持
   - 同时启用Webhook和轮询
2. [ ] 故障切换
   - Webhook失败自动启用轮询
3. [ ] 监控指标
   - 记录轮询统计
4. [ ] 集成测试

**验收标准**:
- Webhook和轮询可以共存
- 去重机制正常工作
- 故障切换自动触发

## 8. 配置示例

### 开发环境 (纯轮询)

```bash
# .env
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=60  # 1分钟 (开发环境可以更频繁)
ISSUE_POLLING_REQUIRED_LABELS=vibe-auto
ISSUE_POLLING_BOT_USERNAME=gitautodev-bot
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=7
```

### 生产环境 (Webhook + 轮询备份)

```bash
# .env
# Webhook为主
WEBHOOK_ENABLED=true

# 轮询作为备份 (间隔较长)
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=600  # 10分钟
ISSUE_POLLING_REQUIRED_LABELS=vibe-auto
ISSUE_POLLING_BOT_USERNAME=gitautodev-bot
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
```

### 内网环境 (纯轮询)

```bash
# .env
# 禁用Webhook
WEBHOOK_ENABLED=false

# 轮询为主
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300  # 5分钟
ISSUE_POLLING_REQUIRED_LABELS=vibe-auto
ISSUE_POLLING_BOT_USERNAME=gitautodev-bot
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
```

## 9. 监控和告警

### 9.1 关键指标

```rust
pub struct PollingMetrics {
    // 轮询统计
    pub total_polls: u64,
    pub successful_polls: u64,
    pub failed_polls: u64,
    
    // Task创建统计
    pub tasks_created: u64,
    pub duplicate_attempts: u64,
    
    // 性能指标
    pub avg_poll_duration_ms: f64,
    pub max_poll_duration_ms: u64,
    
    // API限流
    pub rate_limit_hits: u64,
}
```

### 9.2 日志示例

```
[INFO] issue_polling: Polling issues for repositories count=50
[INFO] issue_polling: Fetched issues from repository repository_id=1 issue_count=5
[INFO] issue_polling: Created task from issue repository_id=1 issue_number=123
[DEBUG] issue_polling: Task already exists issue_number=124
[WARN] issue_polling: Rate limited, retrying after delay repository_id=2 retry_count=1 delay_secs=120
[INFO] issue_polling: Completed repository polling repository_id=1 new_tasks=3
[INFO] issue_polling: Completed polling all repositories total=50 success=48 errors=2
```

### 9.3 告警规则

1. **轮询失败率 > 10%**: 检查网络或Git平台状态
2. **平均轮询时间 > 5秒**: 性能问题，考虑优化
3. **API限流次数 > 10/小时**: 调整轮询间隔
4. **重复创建尝试 > 100/小时**: 可能存在并发问题

## 10. 优缺点对比

### Webhook方案

**优点**:
- ✅ 实时性好 (秒级)
- ✅ 资源消耗低
- ✅ 不占用API配额

**缺点**:
- ❌ 需要公网URL
- ❌ 配置复杂
- ❌ 可能丢失事件

### 轮询方案

**优点**:
- ✅ 无需公网URL
- ✅ 配置简单
- ✅ 不会丢失事件
- ✅ 适合本地开发

**缺点**:
- ❌ 实时性差 (分钟级)
- ❌ 消耗API配额
- ❌ 资源消耗较高

### 推荐方案

**生产环境**: Webhook (主) + 轮询 (备份)  
**开发环境**: 纯轮询  
**内网环境**: 纯轮询

## 11. 参考资料

- `docs/task-implementation-research.md` - Task功能实现研究
- `backend/src/services/webhook_retry_service.rs` - Webhook重试服务
- `backend/src/git_provider/traits.rs` - GitProvider接口
- `backend/src/entities/repository.rs` - Repository Entity

## 12. 更新日志

- **2026-01-20**: 初始版本
  - 完成轮询方案设计
  - 完成性能优化方案
  - 完成与Webhook协同设计
  - 完成实现计划
