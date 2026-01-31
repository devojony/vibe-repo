# Task API 设计文档

## API 端点设计

### 1. CRUD 操作

#### 1.1 创建 Task
```
POST /api/tasks
Content-Type: application/json

Request Body:
{
  "workspace_id": 1,
  "issue_number": 123,
  "issue_url": "https://git.example.com/owner/repo/issues/123",
  "issue_title": "Fix bug in authentication",
  "issue_body": "Detailed description...",
  "priority": "High",  // High, Medium, Low
  "assigned_agent_id": 1  // Optional
}

Response: 201 Created
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 123,
  "issue_url": "https://git.example.com/owner/repo/issues/123",
  "issue_title": "Fix bug in authentication",
  "issue_body": "Detailed description...",
  "task_status": "pending",
  "priority": "High",
  "assigned_agent_id": 1,
  "branch_name": null,
  "pr_number": null,
  "pr_url": null,
  "error_message": null,
  "retry_count": 0,
  "max_retries": 3,
  "started_at": null,
  "completed_at": null,
  "created_at": "2026-01-21T10:00:00Z",
  "updated_at": "2026-01-21T10:00:00Z"
}
```

#### 1.2 获取 Task 详情
```
GET /api/tasks/:id

Response: 200 OK
{
  "id": 1,
  "workspace_id": 1,
  ...
}
```

#### 1.3 列出 Workspace 的 Tasks
```
GET /api/workspaces/:workspace_id/tasks?status=Pending&priority=High

Query Parameters:
- status: string (optional) - Filter by status
- priority: string (optional) - Filter by priority
- assigned_agent_id: integer (optional) - Filter by agent
- page: integer (optional, default: 1)
- per_page: integer (optional, default: 20, max: 100)

Response: 200 OK
{
  "tasks": [
    {
      "id": 1,
      "workspace_id": 1,
      ...
    }
  ],
  "total": 50,
  "page": 1,
  "per_page": 20,
  "total_pages": 3
}
```

#### 1.4 更新 Task
```
PATCH /api/tasks/:id
Content-Type: application/json

Request Body:
{
  "priority": "Low",
  "assigned_agent_id": 2
}

Response: 200 OK
{
  "id": 1,
  ...
}
```

#### 1.5 删除 Task (软删除)
```
DELETE /api/tasks/:id

Response: 204 No Content
```

### 2. 状态管理操作

#### 2.1 分配 Agent
```
POST /api/tasks/:id/assign
Content-Type: application/json

Request Body:
{
  "agent_id": 1
}

Response: 200 OK
{
  "id": 1,
  "task_status": "assigned",
  "assigned_agent_id": 1,
  ...
}
```

#### 2.2 开始执行
```
POST /api/tasks/:id/start

Response: 200 OK
{
  "id": 1,
  "task_status": "running",
  "started_at": "2026-01-21T10:05:00Z",
  ...
}
```

#### 2.3 标记完成
```
POST /api/tasks/:id/complete
Content-Type: application/json

Request Body:
{
  "pr_number": 456,
  "pr_url": "https://git.example.com/owner/repo/pulls/456",
  "branch_name": "fix/auth-bug"
}

Response: 200 OK
{
  "id": 1,
  "task_status": "completed",
  "pr_number": 456,
  "pr_url": "https://git.example.com/owner/repo/pulls/456",
  "branch_name": "fix/auth-bug",
  "completed_at": "2026-01-21T10:30:00Z",
  ...
}
```

#### 2.4 标记失败
```
POST /api/tasks/:id/fail
Content-Type: application/json

Request Body:
{
  "error_message": "Failed to create branch",
  "error_type": "GitError"  // Optional
}

Response: 200 OK
{
  "id": 1,
  "task_status": "failed",  // or "pending" if retry_count < max_retries
  "error_message": "Failed to create branch",
  "retry_count": 1,
  ...
}
```

#### 2.5 重试任务
```
POST /api/tasks/:id/retry

Response: 200 OK
{
  "id": 1,
  "task_status": "pending",
  "retry_count": 1,
  "error_message": null,
  ...
}
```

#### 2.6 取消任务
```
POST /api/tasks/:id/cancel

Response: 200 OK
{
  "id": 1,
  "task_status": "cancelled",
  ...
}
```

## 数据模型

### TaskStatus 枚举

```rust
pub enum TaskStatus {
    Pending,    // 待处理
    Assigned,   // 已分配 Agent
    Running,    // 执行中
    Completed,  // 已完成
    Failed,     // 失败 (达到最大重试次数)
    Cancelled,  // 已取消
}
```

**状态转换规则：**
- `Pending` → `Assigned`, `Cancelled`
- `Assigned` → `Running`, `Cancelled`
- `Running` → `Completed`, `Failed`, `Cancelled`
- `Failed` → `Pending` (仅重试，如果 retry_count < max_retries)
- `Completed` 和 `Cancelled` 是终态，不能再转换

**API 序列化：** 枚举值在 API 响应中序列化为小写字符串（如 "pending", "running", "completed"）

### TaskPriority 枚举
```rust
pub enum TaskPriority {
    Low,
    Medium,
    High,
}
```

### CreateTaskRequest
```rust
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    pub issue_number: i32,
    pub issue_url: String,
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub priority: String,
    pub assigned_agent_id: Option<i32>,
}
```

### UpdateTaskRequest
```rust
pub struct UpdateTaskRequest {
    pub priority: Option<String>,
    pub assigned_agent_id: Option<i32>,
}
```

### TaskResponse
```rust
pub struct TaskResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub issue_number: i32,
    pub issue_url: String,
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub task_status: String,  // TaskStatus enum serialized as lowercase string
    pub priority: String,
    pub assigned_agent_id: Option<i32>,
    pub branch_name: Option<String>,
    pub pr_number: Option<i32>,
    pub pr_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### TaskListResponse
```rust
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}
```

## 错误响应

### 400 Bad Request

**无效的状态转换：**
```json
{
  "error": "Invalid state transition",
  "code": "INVALID_STATE_TRANSITION",
  "details": {
    "current_status": "completed",
    "target_status": "failed",
    "allowed_transitions": []
  }
}
```

**其他验证错误：**
```json
{
  "error": "Invalid request",
  "code": "VALIDATION_ERROR",
  "details": {
    "field": "priority",
    "message": "Priority must be one of: Low, Medium, High"
  }
}
```

### 404 Not Found
```json
{
  "error": "Task not found",
  "code": "TASK_NOT_FOUND"
}
```

### 409 Conflict
```json
{
  "error": "Task already exists for this issue",
  "code": "TASK_ALREADY_EXISTS"
}
```

### 422 Unprocessable Entity

**已弃用：** 状态转换错误现在返回 400 Bad Request（见上文）。此错误码保留用于其他业务逻辑错误。

