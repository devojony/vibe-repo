## ADDED Requirements

### Requirement: System SHALL define permission policy
The system SHALL implement configurable permission policy for agent operations.

#### Scenario: Define default policy
- **WHEN** system starts
- **THEN** default policy SHALL allow read operations
- **THEN** default policy SHALL allow write operations within workspace
- **THEN** default policy SHALL allow safe shell commands
- **THEN** default policy SHALL deny delete operations

#### Scenario: Load repository-specific policy
- **WHEN** repository has custom permission policy
- **THEN** system SHALL load policy from repository configuration
- **THEN** system SHALL merge with default policy
- **THEN** repository policy SHALL override defaults

#### Scenario: Validate policy configuration
- **WHEN** policy is loaded
- **THEN** system SHALL validate policy syntax
- **THEN** system SHALL reject invalid policies
- **THEN** system SHALL log validation errors

### Requirement: System SHALL evaluate permission requests
The system SHALL evaluate each permission request against configured policy.

#### Scenario: Evaluate read permission
- **WHEN** agent requests read permission
- **THEN** system SHALL check if path is within workspace
- **THEN** system SHALL allow if path is safe
- **THEN** system SHALL deny if path is outside workspace or sensitive

#### Scenario: Evaluate write permission
- **WHEN** agent requests write permission
- **THEN** system SHALL check if path is within workspace
- **THEN** system SHALL check if path is not protected (.git, etc.)
- **THEN** system SHALL allow if all checks pass
- **THEN** system SHALL deny otherwise

#### Scenario: Evaluate execute permission
- **WHEN** agent requests execute permission
- **THEN** system SHALL extract command name
- **THEN** system SHALL check against command allowlist
- **THEN** system SHALL allow if command is safe (git, cargo, npm, etc.)
- **THEN** system SHALL deny if command is dangerous (rm, dd, mkfs, etc.)

#### Scenario: Evaluate delete permission
- **WHEN** agent requests delete permission
- **THEN** system SHALL automatically deny
- **THEN** system SHALL log denied request with reason

### Requirement: System SHALL respond to permission requests
The system SHALL send permission responses to agent via ACP protocol.

#### Scenario: Send allow response
- **WHEN** permission is granted
- **THEN** system SHALL send permissionResponse with allow=true
- **THEN** agent SHALL proceed with operation

#### Scenario: Send deny response
- **WHEN** permission is denied
- **THEN** system SHALL send permissionResponse with allow=false
- **THEN** system SHALL include denial reason
- **THEN** agent SHALL skip operation

#### Scenario: Handle permission timeout
- **WHEN** permission evaluation exceeds timeout
- **THEN** system SHALL automatically deny
- **THEN** system SHALL log timeout event

### Requirement: System SHALL log permission decisions
The system SHALL maintain audit log of all permission requests and decisions.

#### Scenario: Log permission request
- **WHEN** agent requests permission
- **THEN** system SHALL log request timestamp
- **THEN** system SHALL log tool kind (read/write/execute/delete)
- **THEN** system SHALL log requested path or command
- **THEN** system SHALL log task_id and agent_id

#### Scenario: Log permission decision
- **WHEN** permission is evaluated
- **THEN** system SHALL log decision (allow/deny)
- **THEN** system SHALL log reason for decision
- **THEN** system SHALL log policy rule that matched

#### Scenario: Store permission log
- **WHEN** permission is processed
- **THEN** system SHALL append to tasks.events JSONB array
- **THEN** system SHALL include all permission details
- **THEN** log SHALL be queryable for security audit

### Requirement: System SHALL support permission policy types
The system SHALL support different policy types for different security levels.

#### Scenario: Restrictive policy
- **WHEN** repository uses restrictive policy
- **THEN** only read operations SHALL be allowed
- **THEN** all write and execute operations SHALL be denied
- **THEN** agent SHALL operate in read-only mode

#### Scenario: Standard policy
- **WHEN** repository uses standard policy
- **THEN** read and workspace write operations SHALL be allowed
- **THEN** safe shell commands SHALL be allowed
- **THEN** delete operations SHALL be denied

#### Scenario: Permissive policy
- **WHEN** repository uses permissive policy
- **THEN** all operations within workspace SHALL be allowed
- **THEN** operations outside workspace SHALL still be denied
- **THEN** dangerous commands SHALL still be denied

### Requirement: System SHALL provide permission override
The system SHALL allow manual permission override for special cases.

#### Scenario: Override via configuration
- **WHEN** specific operation needs to be allowed
- **THEN** administrator SHALL add override rule to policy
- **THEN** system SHALL apply override before default rules
- **THEN** override SHALL be logged for audit

#### Scenario: Temporary override
- **WHEN** temporary permission is needed
- **THEN** administrator SHALL set time-limited override
- **THEN** system SHALL apply override until expiration
- **THEN** system SHALL revert to default policy after expiration

### Requirement: System SHALL detect permission abuse
The system SHALL monitor for suspicious permission patterns.

#### Scenario: Detect excessive denials
- **WHEN** agent receives many denied permissions
- **THEN** system SHALL log warning
- **THEN** system SHALL alert administrator
- **THEN** system SHALL consider task as potentially malicious

#### Scenario: Detect unusual patterns
- **WHEN** agent requests unusual permission sequence
- **THEN** system SHALL flag for review
- **THEN** system SHALL log detailed audit trail
- **THEN** system SHALL optionally pause task for manual review
