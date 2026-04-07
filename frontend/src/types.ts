export type TaskStatus = 'Pending' | 'Doing' | 'Finished' | 'Reviewing' | 'Done'

export interface Subtask {
  id: number
  task_id: number
  title: string
  completed: boolean
  status: TaskStatus
  assignee?: string
}

export interface Task {
  id: number
  task_id: string
  title: string
  description: string
  completed: boolean
  status: TaskStatus
  assignee?: string
  created_at: string
  subtasks: Subtask[]
}

// Legacy agent interface, we'll keep it for TasksView assignee dropdown for now, or just use Employee
export interface Agent {
  id: number
  name: string
  cli: string
  system_prompt: string
  work_dir: string
  model?: string
  max_concurrency: number
  created_at: string
}

export type EmployeeStatus = 'idle' | 'working' | 'offline'

export interface Employee {
  id: number
  name: string
  role: string
  department: string
  agent_backend: string
  system_prompt?: string
  status: EmployeeStatus
  created_at: string
}

export interface Execution {
  id: number
  task_id: number
  employee_id: number
  started_at: string
  finished_at?: string
  exit_code?: number
  status: 'running' | 'completed' | 'failed' | 'cancelled'
}
