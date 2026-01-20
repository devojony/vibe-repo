# 容器生命周期自动管理 - 设计方案

**日期**: 2026-01-20  
**版本**: v0.3.0 (计划)  
**状态**: 设计中  
**作者**: Claude Sonnet 4.5

---

## 目录

- [1. 概述](#1-概述)
- [2. 设计理念](#2-设计理念)
- [3. MVP 功能范围](#3-mvp-功能范围)
- [4. Dockerfile 和镜像管理](#4-dockerfile-和镜像管理)
- [5. 数据库 Schema 设计](#5-数据库-schema-设计)
- [6. API 设计](#6-api-设计)
- [7. Service 层设计](#7-service-层设计)
- [8. 实现计划](#8-实现计划)

---

## 1. 概述

### 1.1 背景

当前 VibeRepo 的 Workspace 模块已经实现了基础的容器创建和 init_script 功能（v0.2.0），但容器的生命周期管理还不完善：

**已实现：**
- ✅ 创建 workspace 时创建容器
- ✅ 启动容器
- ✅ 执行 init_script
- ✅ 基础的健康检查服务

**缺失：**
- ❌ 容器异常时自动重启
- ❌ 统一的 workspace 镜像管理
- ❌ 容器状态的独立管理
- ❌ 实时资源监控
- ❌ 手动重启接口

### 1.2 目标

本次设计的目标是实现 **Phase 1: 容器生命周期管理增强**，让系统能够：

1. **自动管理容器生命周期** - 用户无需关心底层容器操作
2. **自动故障恢复** - 容器异常时自动重启（带重试限制）
3. **统一镜像管理** - 提供标准的 workspace 镜像和管理接口
4. **状态可观测** - 清晰的容器状态和健康信息
5. **最小手动干预** - 仅保留必要的手动操作接口

### 1.3 非目标（后续版本）

以下功能不在本次 MVP 范围内：

- ❌ 容器日志 API（Phase 2）
- ❌ 历史数据存储（Phase 2）
- ❌ 自动暂停/恢复策略（Phase 2）
- ❌ 批量操作（Phase 2）
- ❌ WebSocket 实时推送（Phase 3）
- ❌ 多容器支持（Phase 3+）

---

## 2. 设计理念

### 2.1 核心原则

**容器是 Workspace 的实现细节，应该由系统自动管理。**

用户只需关心 Workspace 的业务状态（创建、使用、删除），而不是底层容器操作（启动、停止、重启）。

### 2.2 自动化优先

系统应该自动处理容器的整个生命周期：

```
用户创建 Workspace
  ↓
系统自动：检查镜像 → 构建镜像（如需要）→ 创建容器 → 启动容器 → 执行 init_script
  ↓
健康检查服务：持续监控容器状态
  ↓
容器异常？→ 自动重启（最多3次）→ 失败则标记为 Failed
  ↓
用户删除 Workspace
  ↓
系统自动：停止容器 → 删除容器 → 清理数据
```

### 2.3 优雅降级

当 Docker 不可用时，系统应该：
- 允许创建 workspace（但标记为无容器状态）
- API 返回明确的错误信息
- 不影响其他功能的正常运行

---

## 3. MVP 功能范围

### 3.1 核心功能

#### 3.1.1 自动化容器生命周期

- **创建时**：自动检查镜像 → 构建（如需要）→ 创建并启动容器
- **运行时**：健康检查服务持续监控，异常时自动重启
- **删除时**：自动停止并删除容器

#### 3.1.2 统一镜像管理

- 提供标准的 `docker/workspace/Dockerfile`
- 首次创建 workspace 时自动构建镜像
- 后续 workspace 复用已构建的镜像
- 提供镜像管理 API（查询、删除、重建）

#### 3.1.3 容器状态独立管理

- 创建独立的 `containers` 表
- 与 `workspaces` 表一对一关联
- 清晰的状态管理和历史记录

#### 3.1.4 故障恢复

- 自动重启：最多 3 次
- 手动重启：提供 API 接口
- 失败标记：超过重试次数后标记为 Failed

#### 3.1.5 实时资源监控

- 提供 API 查询容器实时资源使用情况
- CPU、内存使用率
- 网络流量统计

### 3.2 API 端点

#### Workspace 操作
- `POST /api/workspaces/:id/restart` - 手动重启容器
- `GET /api/workspaces/:id/stats` - 获取实时资源使用情况

#### 镜像管理（Settings）
- `GET /api/settings/workspace/image` - 查询镜像信息
- `DELETE /api/settings/workspace/image` - 删除镜像
- `POST /api/settings/workspace/image/rebuild` - 重新构建镜像

---

## 4. Dockerfile 和镜像管理

### 4.1 目录结构

```
vibo-repo/
├── docker/
│   └── workspace/
│       ├── Dockerfile          # Workspace 容器镜像定义
│       └── README.md           # 镜像说明和使用文档
├── backend/
│   └── Dockerfile              # 后端服务镜像（已有）
└── docker-compose.yml
```

### 4.2 Dockerfile 内容

**文件路径**: `docker/workspace/Dockerfile`

```dockerfile
FROM ubuntu:22.04

# 设置环境变量
ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# 安装基础工具
RUN apt-get update && apt-get install -y \
    # 版本控制
    git \
    # 网络工具
    curl \
    wget \
    # 编辑器
    vim \
    nano \
    # 构建工具
    build-essential \
    # SSL 证书
    ca-certificates \
    # 其他常用工具
    unzip \
    zip \
    jq \
    && rm -rf /var/lib/apt/lists/*

# 创建工作目录
WORKDIR /workspace

# 保持容器运行
CMD ["sleep", "infinity"]
```

**设计说明：**
- 基于 Ubuntu 22.04 LTS（稳定、广泛支持）
- 仅安装最基础的工具（~200MB）
- 用户通过 `init_script` 安装特定语言和工具
- 保持镜像轻量化和灵活性

### 4.3 镜像管理策略

#### 4.3.1 镜像命名

- **镜像名称**: `vibe-repo-workspace:latest`
- **标签策略**: 当前仅使用 `latest`，未来可扩展版本标签

#### 4.3.2 构建时机

```
创建第一个 Workspace
  ↓
检查镜像是否存在
  ├─ 存在 → 直接使用
  └─ 不存在 → 构建镜像
      ↓
      docker build -f docker/workspace/Dockerfile \
                   -t vibe-repo-workspace:latest \
                   .
      ↓
      构建成功 → 创建容器
      构建失败 → 返回错误
```

#### 4.3.3 镜像复用

- 所有 workspace 共享同一个镜像
- 减少磁盘占用
- 加快 workspace 创建速度

#### 4.3.4 镜像更新流程

```
用户更新 Dockerfile
  ↓
调用 DELETE /api/settings/workspace/image
  ↓
系统检查是否有容器使用该镜像
  ├─ 有 → 返回错误（需要先删除所有 workspace）
  └─ 无 → 删除镜像
      ↓
      调用 POST /api/settings/workspace/image/rebuild
      ↓
      重新构建镜像
      ↓
      后续创建的 workspace 使用新镜像
```

### 4.4 构建参数

**构建命令：**
```bash
docker build \
  -f docker/workspace/Dockerfile \
  -t vibe-repo-workspace:latest \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  .
```

**构建上下文**: 项目根目录（`.`）

**构建时间**: 预计 30-60 秒（首次，取决于网络速度）

---

## 5. 数据库 Schema 设计

### 5.1 设计原则

**关注点分离**：
- `workspaces` 表：业务逻辑和配置
- `containers` 表：容器技术实现和状态

**一对一关系**：
- 一个 workspace 对应一个 container
- 通过外键关联，级联删除

### 5.2 表结构

#### 5.2.1 workspaces 表（简化）

```sql
CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id INTEGER NOT NULL UNIQUE,
    workspace_status TEXT NOT NULL,  -- 'Initializing', 'Active', 'Failed', 'Deleted'
    
    -- 资源配置
    image_source TEXT NOT NULL DEFAULT 'vibe-repo-workspace:latest',
    max_concurrent_tasks INTEGER NOT NULL DEFAULT 3,
    cpu_limit REAL NOT NULL DEFAULT 2.0,
    memory_limit TEXT NOT NULL DEFAULT '4GB',
    disk_limit TEXT NOT NULL DEFAULT '10GB',
    work_dir TEXT,
    
    -- 时间戳
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
```

**移除的字段**（迁移到 containers 表）：
- ~~container_id~~
- ~~container_status~~
- ~~health_status~~
- ~~last_health_check~~

**workspace_status 状态值：**
- `Initializing` - 正在初始化（创建容器中）
- `Active` - 活跃状态（容器正常运行）
- `Failed` - 失败状态（容器启动失败或超过重启次数）
- `Deleted` - 已删除（软删除）

#### 5.2.2 containers 表（新增）

```sql
CREATE TABLE containers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL UNIQUE,  -- 一对一关系
    
    -- 容器基本信息
    container_id TEXT NOT NULL UNIQUE,  -- Docker 容器 ID
    container_name TEXT NOT NULL,       -- workspace-{workspace_id}
    image_name TEXT NOT NULL,           -- vibe-repo-workspace:latest
    image_id TEXT,                      -- Docker 镜像 ID (sha256:...)
    
    -- 容器状态
    status TEXT NOT NULL,               -- 'creating', 'running', 'stopped', 'exited', 'failed'
    health_status TEXT,                 -- 'Healthy', 'Unhealthy', 'Unknown'
    exit_code INTEGER,                  -- 容器退出码
    error_message TEXT,                 -- 错误信息
    
    -- 重启管理
    restart_count INTEGER NOT NULL DEFAULT 0,
    max_restart_attempts INTEGER NOT NULL DEFAULT 3,
    last_restart_at TIMESTAMP,
    
    -- 健康检查
    last_health_check TIMESTAMP,
    health_check_failures INTEGER NOT NULL DEFAULT 0,
    
    -- 时间戳
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,               -- 容器启动时间
    stopped_at TIMESTAMP,               -- 容器停止时间
    
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX idx_containers_workspace_id ON containers(workspace_id);
CREATE INDEX idx_containers_status ON containers(status);
CREATE UNIQUE INDEX idx_containers_container_id ON containers(container_id);
```

**status 状态值：**
- `creating` - 正在创建
- `running` - 运行中
- `stopped` - 已停止
- `exited` - 已退出
- `failed` - 失败

**health_status 状态值：**
- `Healthy` - 健康
- `Unhealthy` - 不健康
- `Unknown` - 未知（未检查或检查失败）

### 5.3 关系图

```
repositories (1) ──→ (1) workspaces (1) ──→ (1) containers
```

**关系说明：**
- 一个 repository 对应一个 workspace（已有）
- 一个 workspace 对应一个 container（一对一，UNIQUE 约束）
- 删除 workspace 时自动删除 container（CASCADE）

### 5.4 数据迁移

#### 5.4.1 迁移策略

**现有 workspaces 表的容器字段：**
- `container_id` - 迁移到 containers 表
- `container_status` - 迁移到 containers 表的 `status`
- `health_status` - 迁移到 containers 表
- `last_health_check` - 迁移到 containers 表

**迁移步骤：**
1. 创建 `containers` 表
2. 将现有 workspace 的容器数据迁移到 `containers` 表
3. 删除 workspaces 表中的容器相关字段

#### 5.4.2 迁移 SQL

```sql
-- 步骤 1: 创建 containers 表（见上面的 CREATE TABLE）

-- 步骤 2: 迁移现有数据
INSERT INTO containers (
    workspace_id, 
    container_id, 
    container_name, 
    image_name, 
    status, 
    health_status,
    last_health_check,
    created_at,
    updated_at
)
SELECT 
    id as workspace_id,
    container_id,
    'workspace-' || id as container_name,
    image_source as image_name,
    COALESCE(container_status, 'unknown') as status,
    health_status,
    last_health_check,
    created_at,
    updated_at
FROM workspaces
WHERE container_id IS NOT NULL;

-- 步骤 3: 删除 workspaces 表中的旧字段
ALTER TABLE workspaces DROP COLUMN container_id;
ALTER TABLE workspaces DROP COLUMN container_status;
ALTER TABLE workspaces DROP COLUMN health_status;
ALTER TABLE workspaces DROP COLUMN last_health_check;
```

**注意**: SQLite 不支持 `DROP COLUMN`，需要使用重建表的方式。

### 5.5 SeaORM Entity 定义

#### 5.5.1 Container Entity

**文件路径**: `backend/src/entities/container.rs`

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "containers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub workspace_id: i32,
    pub container_id: String,
    pub container_name: String,
    pub image_name: String,
    pub image_id: Option<String>,
    pub status: String,
    pub health_status: Option<String>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub restart_count: i32,
    pub max_restart_attempts: i32,
    pub last_restart_at: Option<DateTimeUtc>,
    pub last_health_check: Option<DateTimeUtc>,
    pub health_check_failures: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub started_at: Option<DateTimeUtc>,
    pub stopped_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workspace::Entity",
        from = "Column::WorkspaceId",
        to = "super::workspace::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Workspace,
}

impl Related<super::workspace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workspace.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

#### 5.5.2 Workspace Entity 更新

**更新关系定义**：

```rust
// 在 workspace.rs 中添加
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // ... 现有关系 ...
    
    #[sea_orm(has_one = "super::container::Entity")]
    Container,
}

impl Related<super::container::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Container.def()
    }
}
```

---

## 6. API 设计

### 6.1 Workspace 操作 API

#### 6.1.1 手动重启容器

```
POST /api/workspaces/:id/restart
```

**用途**: 手动重启容器（用于故障恢复）

**请求体**: 无

**响应（成功）**:
```json
{
  "message": "Container restarted successfully",
  "workspace_id": 1,
  "container": {
    "id": 1,
    "container_id": "abc123...",
    "status": "running",
    "restart_count": 1,
    "last_restart_at": "2026-01-20T10:30:00Z"
  }
}
```

**响应（失败 - 容器不存在）**:
```json
{
  "error": "Container not found for workspace 1"
}
```

**响应（失败 - 重启失败）**:
```json
{
  "error": "Failed to restart container: Docker daemon not responding"
}
```

**状态码**:
- `200 OK` - 重启成功
- `404 Not Found` - Workspace 或容器不存在
- `500 Internal Server Error` - 重启失败

---

#### 6.1.2 获取实时资源使用情况

```
GET /api/workspaces/:id/stats
```

**用途**: 查询容器实时资源使用情况

**请求参数**: 无

**响应（成功）**:
```json
{
  "workspace_id": 1,
  "container_id": "abc123...",
  "stats": {
    "cpu_percent": 15.5,
    "memory_usage_mb": 512.3,
    "memory_limit_mb": 4096.0,
    "memory_percent": 12.5,
    "network_rx_bytes": 1048576,
    "network_tx_bytes": 524288
  },
  "collected_at": "2026-01-20T10:30:00Z"
}
```

**响应（容器未运行）**:
```json
{
  "error": "Container is not running",
  "workspace_id": 1,
  "container_status": "stopped"
}
```

**状态码**:
- `200 OK` - 查询成功
- `404 Not Found` - Workspace 不存在
- `409 Conflict` - 容器未运行
- `500 Internal Server Error` - 查询失败

---

### 6.2 镜像管理 API（Settings）

#### 6.2.1 查询镜像信息

```
GET /api/settings/workspace/image
```

**用途**: 查看当前 workspace 镜像状态

**请求参数**: 无

**响应（镜像存在）**:
```json
{
  "exists": true,
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:abc123...",
  "size_mb": 234.5,
  "created_at": "2026-01-20T10:00:00Z",
  "in_use_by_workspaces": 3,
  "workspace_ids": [1, 2, 5]
}
```

**响应（镜像不存在）**:
```json
{
  "exists": false,
  "image_name": "vibe-repo-workspace:latest",
  "message": "Image will be built automatically when creating the first workspace"
}
```

**状态码**:
- `200 OK` - 查询成功

---

#### 6.2.2 删除镜像

```
DELETE /api/settings/workspace/image
```

**用途**: 删除现有的 workspace 镜像

**请求参数**: 无

**响应（成功）**:
```json
{
  "message": "Workspace image deleted successfully",
  "image_name": "vibe-repo-workspace:latest"
}
```

**响应（镜像不存在）**:
```json
{
  "message": "Workspace image does not exist",
  "image_name": "vibe-repo-workspace:latest"
}
```

**响应（有容器使用中）**:
```json
{
  "error": "Cannot delete image: 3 workspaces are using this image",
  "active_workspace_ids": [1, 2, 5],
  "suggestion": "Delete all workspaces first, then delete the image"
}
```

**状态码**:
- `200 OK` - 删除成功或镜像不存在
- `409 Conflict` - 有容器正在使用该镜像
- `500 Internal Server Error` - 删除失败

---

#### 6.2.3 重新构建镜像

```
POST /api/settings/workspace/image/rebuild
```

**用途**: 重新构建 workspace 镜像

**请求体**:
```json
{
  "force": false  // 可选，默认 false
}
```

**参数说明**:
- `force`: 是否强制重建（即使有容器使用该镜像）

**响应（成功）**:
```json
{
  "message": "Workspace image rebuilt successfully",
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:def456...",
  "build_time_seconds": 45.3,
  "size_mb": 234.5
}
```

**响应（有容器使用且 force=false）**:
```json
{
  "error": "Cannot rebuild image: 3 workspaces are using this image",
  "active_workspace_ids": [1, 2, 5],
  "suggestion": "Use force=true to rebuild anyway, or delete all workspaces first",
  "warning": "Force rebuild will not affect running containers, but they may behave unexpectedly"
}
```

**响应（构建失败）**:
```json
{
  "error": "Failed to build workspace image",
  "details": "Dockerfile syntax error at line 15: unknown instruction: INSTLL"
}
```

**状态码**:
- `200 OK` - 构建成功
- `409 Conflict` - 有容器使用且未设置 force
- `500 Internal Server Error` - 构建失败

---

### 6.3 API 路由结构

```
/api/
├── workspaces/
│   ├── POST /                      - 创建 workspace（已有）
│   ├── GET /                       - 列出 workspaces（已有）
│   ├── GET /:id                    - 获取 workspace（已有）
│   ├── PATCH /:id/status           - 更新状态（已有）
│   ├── DELETE /:id                 - 删除 workspace（已有）
│   ├── POST /:id/restart           - 手动重启容器（新增）
│   └── GET /:id/stats              - 获取资源使用情况（新增）
│
└── settings/
    ├── providers/                  - RepoProvider 配置（已有）
    └── workspace/                  - Workspace 全局设置（新增）
        └── image/
            ├── GET                 - 查询镜像信息
            ├── DELETE              - 删除镜像
            └── rebuild/
                └── POST            - 重新构建镜像
```

---

## 7. Service 层设计

### 7.1 ContainerService（新增）

创建专门的 `ContainerService` 来管理容器的 CRUD 操作和状态管理。

**文件路径**: `backend/src/services/container_service.rs`

```rust
use crate::entities::{prelude::*, container};
use crate::error::{VibeRepoError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

#[derive(Clone)]
pub struct ContainerService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl ContainerService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }

    /// 创建容器记录并启动 Docker 容器
    pub async fn create_and_start_container(
        &self,
        workspace_id: i32,
        image_name: &str,
        cpu_limit: f64,
        memory_limit: &str,
    ) -> Result<container::Model>;

    /// 获取容器信息
    pub async fn get_container_by_workspace_id(
        &self,
        workspace_id: i32,
    ) -> Result<Option<container::Model>>;

    /// 更新容器状态
    pub async fn update_container_status(
        &self,
        container_id: i32,
        status: &str,
        health_status: Option<&str>,
    ) -> Result<container::Model>;

    /// 自动重启容器（健康检查服务调用）
    pub async fn auto_restart_container(&self, container_id: i32) -> Result<()>;

    /// 手动重启容器（API 调用）
    pub async fn manual_restart_container(&self, container_id: i32) -> Result<container::Model>;

    /// 停止并删除容器
    pub async fn stop_and_remove_container(&self, container_id: i32) -> Result<()>;

    /// 增加重启计数
    async fn increment_restart_count(&self, container_id: i32) -> Result<i32>;

    /// 重置重启计数
    async fn reset_restart_count(&self, container_id: i32) -> Result<()>;

    /// 标记为失败
    async fn mark_as_failed(&self, container_id: i32, error_message: &str) -> Result<()>;
}
```

---

### 7.2 DockerService 增强

在现有的 `DockerService` 中添加新方法。

**文件路径**: `backend/src/services/docker_service.rs`

#### 7.2.1 镜像管理方法

```rust
impl DockerService {
    /// 检查镜像是否存在
    pub async fn image_exists(&self, image_name: &str) -> Result<bool> {
        use bollard::image::ListImagesOptions;
        
        let options = ListImagesOptions {
            filters: vec![("reference", vec![image_name])].into_iter().collect(),
            ..Default::default()
        };
        
        let images = self.docker.list_images(Some(options)).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list images: {}", e)))?;
        
        Ok(!images.is_empty())
    }

    /// 构建镜像
    pub async fn build_image(
        &self,
        dockerfile_path: &str,
        image_name: &str,
        context_path: &str,
    ) -> Result<BuildImageResult> {
        use bollard::image::BuildImageOptions;
        use futures::StreamExt;
        use std::path::Path;
        use tokio::time::Instant;
        
        let start_time = Instant::now();
        
        // 读取 Dockerfile 内容
        let dockerfile_content = tokio::fs::read_to_string(dockerfile_path).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to read Dockerfile: {}", e)))?;
        
        // 创建 tar 归档（包含 Dockerfile 和上下文）
        let tar = create_build_context(context_path, &dockerfile_content)?;
        
        let options = BuildImageOptions {
            dockerfile: "Dockerfile",
            t: image_name,
            rm: true,
            ..Default::default()
        };
        
        let mut stream = self.docker.build_image(options, None, Some(tar.into()));
        
        // 处理构建输出
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(output) => {
                    tracing::debug!("Build output: {:?}", output);
                }
                Err(e) => {
                    return Err(VibeRepoError::Internal(format!("Build failed: {}", e)));
                }
            }
        }
        
        let build_time = start_time.elapsed().as_secs_f64();
        
        // 获取镜像信息
        let image_info = self.inspect_image(image_name).await?;
        
        Ok(BuildImageResult {
            image_name: image_name.to_string(),
            image_id: image_info.id,
            build_time_seconds: build_time,
            size_bytes: image_info.size_bytes,
        })
    }

    /// 删除镜像
    pub async fn remove_image(&self, image_name: &str, force: bool) -> Result<()> {
        use bollard::image::RemoveImageOptions;
        
        let options = RemoveImageOptions {
            force,
            ..Default::default()
        };
        
        self.docker.remove_image(image_name, Some(options), None).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to remove image: {}", e)))?;
        
        Ok(())
    }

    /// 获取镜像信息
    pub async fn inspect_image(&self, image_name: &str) -> Result<ImageInfo> {
        let inspect = self.docker.inspect_image(image_name).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to inspect image: {}", e)))?;
        
        Ok(ImageInfo {
            id: inspect.id.unwrap_or_default(),
            name: image_name.to_string(),
            size_bytes: inspect.size.unwrap_or(0),
            created_at: parse_docker_timestamp(&inspect.created.unwrap_or_default())?,
        })
    }

    /// 列出使用指定镜像的容器
    pub async fn list_containers_using_image(&self, image_name: &str) -> Result<Vec<String>> {
        use bollard::container::ListContainersOptions;
        
        let options = ListContainersOptions {
            all: true,
            filters: vec![("ancestor", vec![image_name])].into_iter().collect(),
            ..Default::default()
        };
        
        let containers = self.docker.list_containers(Some(options)).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list containers: {}", e)))?;
        
        Ok(containers.into_iter()
            .filter_map(|c| c.id)
            .collect())
    }
}
```

#### 7.2.2 容器操作方法

```rust
impl DockerService {
    /// 重启容器
    pub async fn restart_container(&self, container_id: &str, timeout: i64) -> Result<()> {
        use bollard::container::RestartContainerOptions;
        
        let options = RestartContainerOptions { t: timeout };
        
        self.docker.restart_container(container_id, Some(options)).await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to restart container: {}", e)))
    }

    /// 获取容器实时资源使用情况
    pub async fn get_container_stats(&self, container_id: &str) -> Result<ContainerStats> {
        use bollard::container::StatsOptions;
        use futures::StreamExt;
        
        let options = StatsOptions {
            stream: false,
            ..Default::default()
        };
        
        let mut stream = self.docker.stats(container_id, Some(options));
        
        if let Some(stats) = stream.next().await {
            let stats = stats.map_err(|e| 
                VibeRepoError::Internal(format!("Failed to get container stats: {}", e))
            )?;
            
            // 计算 CPU 使用率
            let cpu_percent = calculate_cpu_percent(&stats);
            
            // 计算内存使用
            let memory_usage_mb = stats.memory_stats.usage.unwrap_or(0) as f64 / 1024.0 / 1024.0;
            let memory_limit_mb = stats.memory_stats.limit.unwrap_or(0) as f64 / 1024.0 / 1024.0;
            let memory_percent = if memory_limit_mb > 0.0 {
                (memory_usage_mb / memory_limit_mb) * 100.0
            } else {
                0.0
            };
            
            // 网络统计
            let (network_rx_bytes, network_tx_bytes) = calculate_network_stats(&stats);
            
            Ok(ContainerStats {
                cpu_percent,
                memory_usage_mb,
                memory_limit_mb,
                memory_percent,
                network_rx_bytes,
                network_tx_bytes,
            })
        } else {
            Err(VibeRepoError::Internal("No stats available".to_string()))
        }
    }
}
```

#### 7.2.3 辅助数据结构

```rust
#[derive(Debug, Clone)]
pub struct BuildImageResult {
    pub image_name: String,
    pub image_id: String,
    pub build_time_seconds: f64,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}
```

---

### 7.3 WorkspaceService 更新

更新现有的 `WorkspaceService` 以使用新的 `ContainerService`。

**文件路径**: `backend/src/services/workspace_service.rs`

```rust
use crate::services::{ContainerService, DockerService};

impl WorkspaceService {
    /// 创建 workspace 并启动容器
    pub async fn create_workspace_with_container(
        &self,
        repository_id: i32,
    ) -> Result<(workspace::Model, Option<container::Model>)> {
        // 1. 创建 workspace 记录
        let workspace = self.create_workspace(repository_id).await?;
        
        // 2. 如果 Docker 可用，创建容器
        let container = if self.docker.is_some() {
            let container_service = ContainerService::new(self.db.clone(), self.docker.clone());
            
            // 确保镜像存在
            self.ensure_image_exists(&workspace.image_source).await?;
            
            // 创建并启动容器
            let container = container_service
                .create_and_start_container(
                    workspace.id,
                    &workspace.image_source,
                    workspace.cpu_limit,
                    &workspace.memory_limit,
                )
                .await?;
            
            // 更新 workspace 状态为 Active
            self.update_workspace_status(workspace.id, "Active").await?;
            
            Some(container)
        } else {
            tracing::warn!("Docker not available, workspace created without container");
            None
        };
        
        Ok((workspace, container))
    }

    /// 确保镜像存在（不存在则构建）
    async fn ensure_image_exists(&self, image_name: &str) -> Result<()> {
        let docker = self.docker.as_ref().ok_or_else(|| 
            VibeRepoError::Internal("Docker not available".to_string())
        )?;
        
        // 检查镜像是否存在
        if docker.image_exists(image_name).await? {
            tracing::info!("Image {} already exists", image_name);
            return Ok(());
        }
        
        // 构建镜像
        tracing::info!("Building image {}...", image_name);
        let result = docker.build_image(
            "docker/workspace/Dockerfile",
            image_name,
            ".",
        ).await?;
        
        tracing::info!(
            "Image built successfully: {} ({}s)",
            result.image_name,
            result.build_time_seconds
        );
        
        Ok(())
    }
}
```

---

### 7.4 HealthCheckService 增强

更新健康检查服务以使用新的 `ContainerService`。

**文件路径**: `backend/src/services/health_check_service.rs`

```rust
use crate::services::ContainerService;

impl HealthCheckService {
    /// 检查所有 workspace 的容器健康状态
    pub async fn check_all_workspaces(&self) -> Result<()> {
        let docker = match &self.docker {
            Some(d) => d,
            None => return Ok(()),
        };
        
        let container_service = ContainerService::new(self.db.as_ref().clone(), Some(docker.clone()));
        
        // 获取所有容器
        let containers = Container::find()
            .all(self.db.as_ref())
            .await
            .map_err(VibeRepoError::Database)?;
        
        for container in containers {
            // 检查容器健康状态
            match docker.check_container_health(&container.container_id).await {
                Ok(health) => {
                    // 更新容器状态
                    let status = if health.is_running { "running" } else { "stopped" };
                    let health_status = if health.is_running { "Healthy" } else { "Unhealthy" };
                    
                    if let Err(e) = container_service
                        .update_container_status(container.id, status, Some(health_status))
                        .await
                    {
                        tracing::warn!("Failed to update container {} status: {}", container.id, e);
                    }
                    
                    // 如果容器不健康，尝试自动重启
                    if !health.is_running && container.restart_count < container.max_restart_attempts {
                        tracing::info!("Container {} is unhealthy, attempting auto-restart", container.id);
                        if let Err(e) = container_service.auto_restart_container(container.id).await {
                            tracing::error!("Failed to auto-restart container {}: {}", container.id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check health for container {}: {}", container.id, e);
                }
            }
        }
        
        Ok(())
    }
}
```

---

### 7.5 ImageManagementService（新增）

创建专门的服务来管理 workspace 镜像。

**文件路径**: `backend/src/services/image_management_service.rs`

```rust
use crate::entities::prelude::*;
use crate::error::{VibeRepoError, Result};
use crate::services::DockerService;
use sea_orm::{DatabaseConnection, EntityTrait};

#[derive(Clone)]
pub struct ImageManagementService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl ImageManagementService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }

    /// 获取镜像信息
    pub async fn get_image_info(&self, image_name: &str) -> Result<Option<ImageInfo>> {
        let docker = self.docker.as_ref().ok_or_else(|| 
            VibeRepoError::Internal("Docker not available".to_string())
        )?;
        
        if !docker.image_exists(image_name).await? {
            return Ok(None);
        }
        
        let info = docker.inspect_image(image_name).await?;
        Ok(Some(info))
    }

    /// 获取使用该镜像的 workspace 列表
    pub async fn get_workspaces_using_image(&self, image_name: &str) -> Result<Vec<i32>> {
        let containers = Container::find()
            .filter(container::Column::ImageName.eq(image_name))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;
        
        Ok(containers.into_iter().map(|c| c.workspace_id).collect())
    }

    /// 删除镜像
    pub async fn delete_image(&self, image_name: &str) -> Result<()> {
        let docker = self.docker.as_ref().ok_or_else(|| 
            VibeRepoError::Internal("Docker not available".to_string())
        )?;
        
        // 检查是否有容器使用该镜像
        let workspace_ids = self.get_workspaces_using_image(image_name).await?;
        if !workspace_ids.is_empty() {
            return Err(VibeRepoError::Conflict(format!(
                "Cannot delete image: {} workspaces are using this image",
                workspace_ids.len()
            )));
        }
        
        // 删除镜像
        docker.remove_image(image_name, false).await?;
        
        Ok(())
    }

    /// 重新构建镜像
    pub async fn rebuild_image(
        &self,
        image_name: &str,
        force: bool,
    ) -> Result<BuildImageResult> {
        let docker = self.docker.as_ref().ok_or_else(|| 
            VibeRepoError::Internal("Docker not available".to_string())
        )?;
        
        // 检查是否有容器使用该镜像
        if !force {
            let workspace_ids = self.get_workspaces_using_image(image_name).await?;
            if !workspace_ids.is_empty() {
                return Err(VibeRepoError::Conflict(format!(
                    "Cannot rebuild image: {} workspaces are using this image. Use force=true to rebuild anyway.",
                    workspace_ids.len()
                )));
            }
        }
        
        // 删除旧镜像（如果存在）
        if docker.image_exists(image_name).await? {
            docker.remove_image(image_name, true).await?;
        }
        
        // 重新构建镜像
        let result = docker.build_image(
            "docker/workspace/Dockerfile",
            image_name,
            ".",
        ).await?;
        
        Ok(result)
    }
}
```

---

### 7.6 Service 层总结

**新增的 Service：**
1. `ContainerService` - 容器 CRUD 和生命周期管理
2. `ImageManagementService` - 镜像管理

**增强的 Service：**
1. `DockerService` - 添加镜像和资源监控方法
2. `WorkspaceService` - 集成容器创建和镜像管理
3. `HealthCheckService` - 使用新的容器服务进行健康检查

**Service 依赖关系：**
```
WorkspaceService
  ├─→ ContainerService
  │     └─→ DockerService
  └─→ ImageManagementService
        └─→ DockerService

HealthCheckService
  └─→ ContainerService
        └─→ DockerService
```

---

## 8. 实现计划

### 8.1 实现阶段划分

#### Phase 1: 基础设施（1-2 天）
- 创建 Dockerfile
- 数据库 Migration
- Entity 定义

#### Phase 2: Service 层（2-3 天）
- ContainerService 实现
- DockerService 增强
- ImageManagementService 实现

#### Phase 3: API 层（2-3 天）
- Workspace 操作 API
- 镜像管理 API
- OpenAPI 文档更新

#### Phase 4: 健康检查增强（1-2 天）
- 更新 HealthCheckService
- 自动重启逻辑
- 失败处理

#### Phase 5: 测试和文档（2-3 天）
- 单元测试
- 集成测试
- 用户文档

**总计**: 8-13 个工作日

---

### 8.2 详细任务清单

#### 8.2.1 Phase 1: 基础设施

**任务 1.1: 创建 Dockerfile**
- [ ] 创建 `docker/workspace/` 目录
- [ ] 编写 `docker/workspace/Dockerfile`
- [ ] 编写 `docker/workspace/README.md`
- [ ] 本地测试构建镜像

**任务 1.2: 数据库 Migration**
- [ ] 创建 migration 文件 `m20260120_000001_create_containers_table.rs`
- [ ] 实现 `up()` 方法（创建 containers 表）
- [ ] 实现 `down()` 方法（删除 containers 表）
- [ ] 实现数据迁移逻辑（从 workspaces 迁移容器数据）
- [ ] 测试 migration

**任务 1.3: Entity 定义**
- [ ] 创建 `entities/container.rs`
- [ ] 定义 Container Model
- [ ] 定义 Relation
- [ ] 更新 `entities/workspace.rs` 添加 Container 关系
- [ ] 更新 `entities/prelude.rs`

**验收标准**:
- ✅ Dockerfile 可以成功构建镜像
- ✅ Migration 可以成功执行（up 和 down）
- ✅ Entity 编译通过，关系正确

---

#### 8.2.2 Phase 2: Service 层

**任务 2.1: ContainerService 实现**
- [ ] 创建 `services/container_service.rs`
- [ ] 实现 `create_and_start_container()`
- [ ] 实现 `get_container_by_workspace_id()`
- [ ] 实现 `update_container_status()`
- [ ] 实现 `auto_restart_container()`
- [ ] 实现 `manual_restart_container()`
- [ ] 实现 `stop_and_remove_container()`
- [ ] 实现辅助方法（increment_restart_count 等）
- [ ] 编写单元测试

**任务 2.2: DockerService 增强**
- [ ] 实现 `image_exists()`
- [ ] 实现 `build_image()`
- [ ] 实现 `remove_image()`
- [ ] 实现 `inspect_image()`
- [ ] 实现 `list_containers_using_image()`
- [ ] 实现 `restart_container()`
- [ ] 实现 `get_container_stats()`
- [ ] 实现辅助函数（calculate_cpu_percent 等）
- [ ] 编写单元测试

**任务 2.3: ImageManagementService 实现**
- [ ] 创建 `services/image_management_service.rs`
- [ ] 实现 `get_image_info()`
- [ ] 实现 `get_workspaces_using_image()`
- [ ] 实现 `delete_image()`
- [ ] 实现 `rebuild_image()`
- [ ] 编写单元测试

**任务 2.4: WorkspaceService 更新**
- [ ] 更新 `create_workspace_with_container()` 使用 ContainerService
- [ ] 实现 `ensure_image_exists()`
- [ ] 更新删除逻辑（级联删除容器）
- [ ] 更新单元测试

**任务 2.5: 更新 services/mod.rs**
- [ ] 导出 ContainerService
- [ ] 导出 ImageManagementService
- [ ] 更新 DockerService 导出

**验收标准**:
- ✅ 所有 Service 方法实现完成
- ✅ 单元测试覆盖率 > 80%
- ✅ 所有测试通过

---

#### 8.2.3 Phase 3: API 层

**任务 3.1: Workspace 操作 API**
- [ ] 创建 `api/workspaces/lifecycle_handlers.rs`
- [ ] 实现 `restart_workspace()` handler
- [ ] 实现 `get_workspace_stats()` handler
- [ ] 更新 `api/workspaces/routes.rs` 添加新路由
- [ ] 更新 `api/workspaces/models.rs` 添加响应模型
- [ ] 添加 OpenAPI 文档注解

**任务 3.2: 镜像管理 API**
- [ ] 创建 `api/settings/workspace/` 目录
- [ ] 创建 `api/settings/workspace/mod.rs`
- [ ] 创建 `api/settings/workspace/handlers.rs`
  - [ ] 实现 `get_image_info()` handler
  - [ ] 实现 `delete_image()` handler
  - [ ] 实现 `rebuild_image()` handler
- [ ] 创建 `api/settings/workspace/routes.rs`
- [ ] 创建 `api/settings/workspace/models.rs`
- [ ] 添加 OpenAPI 文档注解

**任务 3.3: 更新 API 模块**
- [ ] 更新 `api/settings/mod.rs` 导入 workspace 模块
- [ ] 更新 `api/mod.rs` 注册新路由
- [ ] 更新 OpenAPI 文档生成

**任务 3.4: 响应模型更新**
- [ ] 更新 `WorkspaceResponse` 包含 container 信息
- [ ] 创建 `ContainerResponse` 模型
- [ ] 创建 `ImageInfoResponse` 模型
- [ ] 创建 `ContainerStatsResponse` 模型

**验收标准**:
- ✅ 所有 API 端点实现完成
- ✅ OpenAPI 文档正确生成
- ✅ 手动测试所有端点正常工作

---

#### 8.2.4 Phase 4: 健康检查增强

**任务 4.1: 更新 HealthCheckService**
- [ ] 更新 `check_all_workspaces()` 使用 ContainerService
- [ ] 实现自动重启逻辑
- [ ] 实现失败标记逻辑
- [ ] 添加重启次数限制检查
- [ ] 更新日志记录

**任务 4.2: 测试健康检查**
- [ ] 编写集成测试（容器正常运行）
- [ ] 编写集成测试（容器异常自动重启）
- [ ] 编写集成测试（超过重启次数标记失败）
- [ ] 手动测试健康检查服务

**验收标准**:
- ✅ 健康检查服务正常运行
- ✅ 自动重启逻辑正确
- ✅ 失败标记逻辑正确
- ✅ 所有测试通过

---

#### 8.2.5 Phase 5: 测试和文档

**任务 5.1: 单元测试**
- [ ] ContainerService 单元测试（10+ 测试用例）
- [ ] DockerService 新方法单元测试（8+ 测试用例）
- [ ] ImageManagementService 单元测试（6+ 测试用例）
- [ ] 确保测试覆盖率 > 80%

**任务 5.2: 集成测试**
- [ ] Workspace 创建流程集成测试
- [ ] 容器重启集成测试
- [ ] 镜像管理集成测试
- [ ] 健康检查集成测试
- [ ] API 端点集成测试

**任务 5.3: 用户文档**
- [ ] 更新 README.md（新增 API 说明）
- [ ] 创建 `docs/container-lifecycle-management.md`
- [ ] 创建 `docker/workspace/README.md`
- [ ] 更新 CHANGELOG.md

**任务 5.4: 代码审查和优化**
- [ ] 代码风格检查（cargo fmt, cargo clippy）
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 日志记录完善

**验收标准**:
- ✅ 测试覆盖率 > 80%
- ✅ 所有测试通过
- ✅ 文档完整清晰
- ✅ 代码质量检查通过

---

### 8.3 测试策略

#### 8.3.1 单元测试

**ContainerService 测试用例：**
1. `test_create_and_start_container_success` - 成功创建并启动容器
2. `test_create_and_start_container_docker_unavailable` - Docker 不可用时的处理
3. `test_get_container_by_workspace_id_found` - 查找容器成功
4. `test_get_container_by_workspace_id_not_found` - 容器不存在
5. `test_update_container_status_success` - 更新状态成功
6. `test_auto_restart_container_success` - 自动重启成功
7. `test_auto_restart_container_max_attempts_exceeded` - 超过最大重启次数
8. `test_manual_restart_container_success` - 手动重启成功
9. `test_stop_and_remove_container_success` - 停止并删除容器成功
10. `test_increment_restart_count` - 增加重启计数

**DockerService 测试用例：**
1. `test_image_exists_true` - 镜像存在
2. `test_image_exists_false` - 镜像不存在
3. `test_build_image_success` - 构建镜像成功
4. `test_build_image_dockerfile_not_found` - Dockerfile 不存在
5. `test_remove_image_success` - 删除镜像成功
6. `test_inspect_image_success` - 查询镜像信息成功
7. `test_restart_container_success` - 重启容器成功
8. `test_get_container_stats_success` - 获取资源统计成功

**ImageManagementService 测试用例：**
1. `test_get_image_info_exists` - 获取镜像信息（存在）
2. `test_get_image_info_not_exists` - 获取镜像信息（不存在）
3. `test_get_workspaces_using_image` - 获取使用镜像的 workspace 列表
4. `test_delete_image_success` - 删除镜像成功
5. `test_delete_image_in_use` - 删除镜像失败（使用中）
6. `test_rebuild_image_success` - 重建镜像成功

#### 8.3.2 集成测试

**文件**: `tests/container_lifecycle_integration_tests.rs`

测试场景：
1. 完整的 workspace 创建流程（包含容器）
2. 容器异常后自动重启
3. 超过重启次数后标记失败
4. 手动重启容器
5. 获取容器资源统计
6. 镜像管理（查询、删除、重建）

#### 8.3.3 手动测试清单

- [ ] 创建第一个 workspace（触发镜像构建）
- [ ] 创建第二个 workspace（复用镜像）
- [ ] 手动停止容器，观察自动重启
- [ ] 手动重启容器 API
- [ ] 查询容器资源统计
- [ ] 查询镜像信息
- [ ] 删除镜像（有容器使用，应该失败）
- [ ] 删除所有 workspace 后删除镜像
- [ ] 重新构建镜像
- [ ] 更新 Dockerfile 后重建镜像

---

### 8.4 风险和缓解措施

#### 风险 1: Docker 不可用
**影响**: 无法创建容器，功能不可用  
**缓解**: 
- 优雅降级，允许创建 workspace 但标记为无容器状态
- 提供清晰的错误信息
- 文档说明 Docker 依赖

#### 风险 2: 镜像构建失败
**影响**: 无法创建 workspace  
**缓解**:
- 详细的构建日志
- 清晰的错误信息
- 提供预构建镜像选项（未来）

#### 风险 3: 容器重启循环
**影响**: 资源浪费，系统不稳定  
**缓解**:
- 设置最大重启次数（默认 3 次）
- 超过次数后标记为 Failed
- 提供手动干预接口

#### 风险 4: 数据迁移失败
**影响**: 现有 workspace 数据丢失  
**缓解**:
- 完整的 migration 测试
- 备份数据库
- 提供回滚方案

#### 风险 5: 性能问题
**影响**: 健康检查占用过多资源  
**缓解**:
- 合理的检查间隔（30 秒）
- 异步处理
- 批量查询优化

---

### 8.5 发布计划

#### 8.5.1 版本号
**v0.3.0** - 容器生命周期管理

#### 8.5.2 发布检查清单
- [ ] 所有测试通过（单元 + 集成）
- [ ] 代码审查完成
- [ ] 文档更新完成
- [ ] CHANGELOG.md 更新
- [ ] README.md 更新
- [ ] Migration 测试通过
- [ ] 手动测试完成
- [ ] 性能测试通过

#### 8.5.3 发布步骤
1. 合并所有功能分支到 main
2. 更新版本号（Cargo.toml, CHANGELOG.md）
3. 创建 git tag `v0.3.0`
4. 推送到远程仓库
5. 创建 GitHub Release
6. 更新文档网站（如有）

---

### 8.6 后续优化（v0.4.0+）

以下功能不在本次 MVP 范围，但可以在后续版本中实现：

**Phase 2 功能：**
- 容器日志 API（查询、流式传输）
- 自动暂停/恢复策略（空闲时暂停）
- 容器快照和恢复
- 批量操作 API

**Phase 3 功能：**
- WebSocket 实时推送（容器状态、资源使用）
- 历史数据存储和趋势分析
- 告警系统（资源超限、容器异常）
- 多容器支持（一个 workspace 多个容器）

**Phase 4 功能：**
- 自定义镜像支持（用户上传 Dockerfile）
- 镜像版本管理
- 容器网络配置
- 卷挂载管理

---

## 9. 总结

### 9.1 设计亮点

1. **关注点分离** - 业务逻辑（workspace）与技术实现（container）分离
2. **自动化优先** - 用户无需关心容器细节，系统自动管理
3. **优雅降级** - Docker 不可用时系统仍可运行
4. **可扩展性** - 清晰的架构便于后续功能扩展
5. **完整的测试** - TDD 方法确保代码质量

### 9.2 技术选型

- **Bollard** - Rust 的 Docker API 客户端
- **SeaORM** - 数据库 ORM
- **Axum** - Web 框架
- **Tokio** - 异步运行时

### 9.3 预期成果

完成本次设计后，VibeRepo 将具备：
- ✅ 完整的容器生命周期自动管理
- ✅ 统一的 workspace 镜像
- ✅ 自动故障恢复（最多 3 次重启）
- ✅ 实时资源监控
- ✅ 灵活的镜像管理

这将为后续的 Agent 集成和 Task 执行功能打下坚实的基础。

---

**文档状态**: 已完成  
**最后更新**: 2026-01-20  
**下一步**: 开始实现 Phase 1
