export type WorkflowStatus =
  | 'Plan'
  | 'Design'
  | 'Coding'
  | 'Review'
  | 'QA'
  | 'NeedsHuman'
  | 'Done'

export type WorkflowBoardGroup = 'Plan' | 'Build' | 'Review' | 'QA' | 'Human' | 'Done'

export type WorkflowRole =
  | 'planner'
  | 'designer'
  | 'coder'
  | 'reviewer'
  | 'qa'
  | 'human'

export type WorkflowAction = 'advance' | 'reject' | 'archive'

export type WorkflowActorType = 'employee' | 'human'

export interface Subtask {
  id: number
  task_id: number
  title: string
  completed: boolean
  status: WorkflowStatus
  assignee?: string | null
}

export interface Task {
  id: number
  task_id: string
  title: string
  description: string
  archived: boolean
  status: WorkflowStatus
  assignee?: string | null
  created_at: string
  subtasks: Subtask[]
}

export interface BoardTaskSummary {
  id: number
  task_id: string
  title: string
  status: WorkflowStatus
  board_group: WorkflowBoardGroup
  assignee?: string | null
  archived: boolean
  needs_attention: boolean
  waiting_for_human: boolean
  rejection_count: number
  latest_event_summary?: string | null
}

export interface WorkflowEvent {
  id: number
  task_id: number
  from_status: WorkflowStatus
  to_status: WorkflowStatus
  actor_type: WorkflowActorType
  actor_id?: number | null
  actor_label: string
  action: WorkflowAction
  note?: string | null
  evidence_text?: string | null
  created_at: string
}

export interface TaskDetail {
  task: Task
  events: WorkflowEvent[]
  current_action_label: string
  current_action_hint?: string | null
}

export interface TransitionPayload {
  actor_type: WorkflowActorType
  actor_id?: number | null
  actor_label?: string | null
  from_status: WorkflowStatus
  to_status?: WorkflowStatus
  action: WorkflowAction
  note?: string | null
  evidence_text?: string | null
}

export interface TransitionResponse {
  task: TaskDetail
}

export interface CreateTaskPayload {
  task_id: string
  title: string
  description: string
  assignee?: string | null
}

export interface UpdateTaskPayload {
  task_id?: string
  title?: string
  description?: string
  assignee?: string | null
}

export interface Agent {
  id: number
  name: string
  cli: string
  system_prompt: string
  work_dir: string
  model?: string | null
  max_concurrency: number
  created_at: string
}

export type EmployeeStatus = 'idle' | 'working' | 'error'

export interface Employee {
  id: number
  name: string
  role: string
  workflow_role: WorkflowRole
  department: string
  agent_backend: string
  backend_available: boolean
  system_prompt?: string | null
  status: EmployeeStatus
  created_at: string
}

export interface EmployeePayload {
  name: string
  role: string
  workflow_role?: WorkflowRole
  department: string
  agent_backend: string
  system_prompt?: string | null
}

export interface Execution {
  id: number
  task_id: number
  employee_id: number
  started_at: string
  finished_at?: string | null
  exit_code?: number | null
  status: 'running' | 'completed' | 'failed' | 'cancelled'
}

export interface CurrentExecution {
  execution_id: number
  task_id: number
  task_key: string
  task_title: string
  started_at: string
}
