import { useEffect, useMemo, useRef, useState } from 'react'
import type {
  BoardTaskSummary,
  CreateTaskPayload,
  Employee,
  TaskDetail,
  TransitionPayload,
  UpdateTaskPayload,
  WorkflowBoardGroup,
  WorkflowRole,
  WorkflowStatus,
} from '../types'
import Modal from './Modal'
import RejectModal from './RejectModal'
import TaskDetailTimeline from './TaskDetailTimeline'
import TransitionActions from './TransitionActions'
import WorkflowBoard from './WorkflowBoard'

type TaskFormState = {
  title: string
  description: string
  source: string
  source_branch: string
  assignee: string
}

const EMPTY_FORM: TaskFormState = {
  title: '',
  description: '',
  source: '',
  source_branch: 'main',
  assignee: '',
}

const GROUP_ORDER: WorkflowBoardGroup[] = ['Plan', 'Build', 'Review', 'QA', 'Human', 'Done']

function requiredWorkflowRole(status: WorkflowStatus): WorkflowRole | null {
  switch (status) {
    case 'Plan':
      return 'planner'
    case 'Design':
      return 'designer'
    case 'Coding':
      return 'coder'
    case 'Review':
      return 'reviewer'
    case 'QA':
      return 'qa'
    default:
      return null
  }
}

function sortGroupTasks(tasks: BoardTaskSummary[]) {
  return [...tasks].sort((left, right) => {
    const leftRank = Number(left.waiting_for_human) * 2 + Number(left.needs_attention)
    const rightRank = Number(right.waiting_for_human) * 2 + Number(right.needs_attention)
    if (leftRank !== rightRank) return rightRank - leftRank
    return left.task_id.localeCompare(right.task_id)
  })
}

function mapBoardGroups(tasks: BoardTaskSummary[]) {
  const groups = {
    Plan: [] as BoardTaskSummary[],
    Build: [] as BoardTaskSummary[],
    Review: [] as BoardTaskSummary[],
    QA: [] as BoardTaskSummary[],
    Human: [] as BoardTaskSummary[],
    Done: [] as BoardTaskSummary[],
  }

  tasks.forEach((task) => {
    groups[task.board_group].push(task)
  })

  return {
    Plan: sortGroupTasks(groups.Plan),
    Build: sortGroupTasks(groups.Build),
    Review: sortGroupTasks(groups.Review),
    QA: sortGroupTasks(groups.QA),
    Human: sortGroupTasks(groups.Human),
    Done: sortGroupTasks(groups.Done),
  }
}

export default function TasksView() {
  const [tasks, setTasks] = useState<BoardTaskSummary[]>([])
  const [employees, setEmployees] = useState<Employee[]>([])
  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null)
  const [selectedTask, setSelectedTask] = useState<TaskDetail | null>(null)
  const [isCompact, setIsCompact] = useState(() => window.innerWidth < 960)
  const [activeGroup, setActiveGroup] = useState<WorkflowBoardGroup>('Human')
  const [actorKey, setActorKey] = useState('')

  const [modalOpen, setModalOpen] = useState(false)
  const [editingDetail, setEditingDetail] = useState<TaskDetail | null>(null)
  const [form, setForm] = useState<TaskFormState>(EMPTY_FORM)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [rejectPayload, setRejectPayload] = useState<TransitionPayload | null>(null)
  const [assignModalOpen, setAssignModalOpen] = useState(false)
  const [assigning, setAssigning] = useState(false)
  const [assignError, setAssignError] = useState<string | null>(null)
  const [publishing, setPublishing] = useState(false)
  const detailPanelRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const onResize = () => setIsCompact(window.innerWidth < 960)
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  const fetchTasks = async () => {
    const response = await fetch('/api/tasks')
    const data = await response.json()
    setTasks(data)
  }

  const fetchEmployees = async () => {
    const response = await fetch('/api/employees')
    const data = await response.json()
    setEmployees(data)
  }

  const fetchTaskDetail = async (taskId: number) => {
    const response = await fetch(`/api/tasks/${taskId}`)
    const data = await response.json()
    setSelectedTask(data)
  }

  useEffect(() => {
    void fetchTasks()
    void fetchEmployees()
  }, [])

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      void fetchTasks()
      void fetchEmployees()
      if (selectedTaskId != null) {
        void fetchTaskDetail(selectedTaskId)
      }
    }, 4000)

    return () => window.clearInterval(intervalId)
  }, [selectedTaskId])

  useEffect(() => {
    if (selectedTaskId == null) {
      setSelectedTask(null)
      return
    }
    void fetchTaskDetail(selectedTaskId)
  }, [selectedTaskId])

  useEffect(() => {
    if (employees.length > 0 && !actorKey) {
      setActorKey(`employee:${employees[0].id}`)
    }
  }, [employees, actorKey])

  useEffect(() => {
    if (!isCompact && selectedTask) {
      detailPanelRef.current?.focus()
    }
  }, [isCompact, selectedTask])

  const groupedTasks = useMemo(() => mapBoardGroups(tasks), [tasks])
  const visibleTaskCount = GROUP_ORDER.reduce((count, group) => count + groupedTasks[group].length, 0)
  const attentionCount = tasks.filter((task) => task.needs_attention || task.waiting_for_human || task.waiting_for_agent).length
  const requiredRole = selectedTask ? requiredWorkflowRole(selectedTask.task.status) : null
  const assignableEmployees = useMemo(
    () => (requiredRole ? employees.filter((employee) => employee.workflow_role === requiredRole) : []),
    [employees, requiredRole],
  )

  const openCreate = () => {
    setEditingDetail(null)
    setForm(EMPTY_FORM)
    setError(null)
    setModalOpen(true)
  }

  const openEdit = () => {
    if (!selectedTask) return
    setEditingDetail(selectedTask)
    setForm({
      title: selectedTask.task.title,
      description: selectedTask.task.description,
      source: selectedTask.task.source ?? '',
      source_branch: selectedTask.task.source_branch ?? 'main',
      assignee: selectedTask.task.assignee ?? '',
    })
    setError(null)
    setModalOpen(true)
  }

  const closeDetail = () => {
    setSelectedTaskId(null)
    setSelectedTask(null)
  }

  const handleTaskSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!form.title.trim()) {
      setError('Title is required.')
      return
    }
    if (!form.source.trim()) {
      setError('Source is required.')
      return
    }
    if (!form.source_branch.trim()) {
      setError('Source branch is required.')
      return
    }

    setSaving(true)
    setError(null)
    try {
      const payload: CreateTaskPayload | UpdateTaskPayload = {
        title: form.title.trim(),
        description: form.description.trim(),
        source: form.source.trim(),
        source_branch: form.source_branch.trim(),
        assignee: form.assignee || null,
      }

      const response = await fetch(editingDetail ? `/api/tasks/${editingDetail.task.id}` : '/api/tasks', {
        method: editingDetail ? 'PUT' : 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      })

      if (response.status === 409) {
        setError('Task ID already exists.')
        return
      }
      if (!response.ok) {
        throw new Error(await response.text())
      }

      const task = await response.json()
      setModalOpen(false)
      await fetchTasks()
      if (editingDetail) {
        setSelectedTaskId(task.id)
      }
    } catch (submitError) {
      setError(submitError instanceof Error ? submitError.message : 'Failed to save task')
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async () => {
    if (!selectedTask) return
    if (!window.confirm(`Delete ${selectedTask.task.task_id}?`)) return

    await fetch(`/api/tasks/${selectedTask.task.id}`, { method: 'DELETE' })
    closeDetail()
    await fetchTasks()
  }

  const openAssignModal = () => {
    if (!selectedTask) return
    setAssignError(null)
    setAssignModalOpen(true)
  }

  const handleAssign = async (employeeId: number) => {
    if (!selectedTask) return
    setAssigning(true)
    setAssignError(null)
    try {
      const response = await fetch(`/api/employees/${employeeId}/assign/${selectedTask.task.id}`, {
        method: 'POST',
      })
      if (!response.ok) {
        const message = await response.text()
        throw new Error(message || `${response.status} ${response.statusText}`)
      }
      setAssignModalOpen(false)
      await fetchTasks()
      await fetchEmployees()
      await fetchTaskDetail(selectedTask.task.id)
    } catch (err) {
      setAssignError(err instanceof Error ? err.message : 'Failed to assign task')
    } finally {
      setAssigning(false)
    }
  }

  const handleTransition = async (payload: TransitionPayload) => {
    if (!selectedTask) return

    const response = await fetch(`/api/tasks/${selectedTask.task.id}/transition`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })

    if (!response.ok) {
      const message = await response.text()
      window.alert(message || 'Transition failed')
      return
    }

    const data = await response.json()
    setSelectedTask(data.task)
    await fetchTasks()
  }

  const handlePublishBranch = async () => {
    if (!selectedTask) return
    setPublishing(true)
    try {
      const response = await fetch(`/api/tasks/${selectedTask.task.id}/publish`, {
        method: 'POST',
      })
      if (!response.ok) {
        const message = await response.text()
        throw new Error(message || 'Push failed')
      }
      const data = await response.json()
      await fetchTaskDetail(selectedTask.task.id)
      window.alert(`Pushed ${data.branch_name}`)
    } catch (err) {
      window.alert(err instanceof Error ? err.message : 'Push failed')
    } finally {
      setPublishing(false)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', minHeight: 0 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '1rem', alignItems: 'flex-start', flexWrap: 'wrap' }}>
        <div>
          <h1 style={{ fontSize: '1.7rem', fontWeight: 750, letterSpacing: '-0.03em' }}>Workflow board</h1>
          <div style={{ marginTop: '0.3rem', color: 'var(--text-secondary)', maxWidth: 640 }}>
            Track one disciplined relay from planning through QA, surface returned work first, and keep human decisions explicit.
          </div>
        </div>
        <button
          type="button"
          onClick={openCreate}
          style={{ background: 'var(--accent-primary)', color: 'white', border: 'none', minHeight: 44 }}
        >
          Create workflow task
        </button>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: isCompact ? '1fr' : '1.8fr minmax(320px, 0.95fr)', gap: '1rem', minHeight: 0 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', minWidth: 0 }}>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '0.75rem' }}>
            <MetricCard label="Active tasks" value={String(visibleTaskCount)} helper="Board summaries only" />
            <MetricCard label="Needs attention" value={String(attentionCount)} helper="Returned + waiting human" accent="rgb(180, 83, 9)" />
            <MetricCard label="Active lanes" value="6" helper="Plan, Build, Review, QA, Human, Done" />
          </div>

          {visibleTaskCount === 0 ? (
            <div
              style={{
                padding: '2rem',
                background: 'var(--surface-elevated)',
                borderRadius: 'var(--radius-xl)',
                border: '1px dashed var(--border-color)',
                display: 'flex',
                flexDirection: 'column',
                gap: '1rem',
                boxShadow: '0 18px 40px -28px rgba(6, 148, 148, 0.45)',
              }}
            >
              <div>
                <div style={{ fontSize: '1.2rem', fontWeight: 700 }}>Start the first relay</div>
                <div style={{ marginTop: '0.35rem', color: 'var(--text-secondary)', maxWidth: 640 }}>
                  This board is not a generic backlog. Work moves through Plan, Build, Review, QA, and a final human decision before it leaves the active workspace.
                </div>
              </div>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.75rem' }}>
                <button
                  type="button"
                  onClick={openCreate}
                  style={{
                    background: 'linear-gradient(180deg, var(--accent-primary), var(--accent-hover))',
                    color: 'var(--accent-contrast)',
                    border: '1px solid color-mix(in srgb, var(--accent-primary) 80%, black 20%)',
                    minHeight: 44,
                    boxShadow: '0 12px 26px -18px rgba(6, 148, 148, 0.9)',
                  }}
                >
                  Create your first workflow task
                </button>
                <button
                  type="button"
                  onClick={() => setActiveGroup('Human')}
                  style={{
                    background: 'var(--button-secondary-bg)',
                    color: 'var(--button-secondary-color)',
                    border: '1px solid var(--button-secondary-border)',
                    minHeight: 44,
                  }}
                >
                  Open example workflow lane
                </button>
              </div>
              <div style={{ fontSize: '0.84rem', color: 'var(--text-tertiary)' }}>
                Review and QA can return work upstream. Only a human can archive finished work.
              </div>
            </div>
          ) : (
            <WorkflowBoard
              groups={groupedTasks}
              selectedTaskId={selectedTaskId}
              onSelectTask={setSelectedTaskId}
              compact={isCompact}
              activeGroup={activeGroup}
              onActiveGroupChange={setActiveGroup}
            />
          )}
        </div>

        {!isCompact && (
          <div ref={detailPanelRef} tabIndex={-1} style={{ minWidth: 0, outline: 'none' }}>
            <TaskDetailShell
              detail={selectedTask}
              employees={employees}
              actorKey={actorKey}
              onActorKeyChange={setActorKey}
              onEdit={openEdit}
              onDelete={handleDelete}
              onClose={closeDetail}
              onTransition={handleTransition}
              onReject={setRejectPayload}
              onAssign={openAssignModal}
              onPublishBranch={handlePublishBranch}
              publishing={publishing}
            />
          </div>
        )}
      </div>

      {isCompact && selectedTask && (
        <Modal open={Boolean(selectedTask)} title={selectedTask.task.title} onClose={closeDetail} width="100%">
          <TaskDetailShell
            detail={selectedTask}
            employees={employees}
            actorKey={actorKey}
            onActorKeyChange={setActorKey}
            onEdit={openEdit}
            onDelete={handleDelete}
            onClose={closeDetail}
            onTransition={handleTransition}
            onReject={setRejectPayload}
            onAssign={openAssignModal}
            onPublishBranch={handlePublishBranch}
            publishing={publishing}
            compact
          />
        </Modal>
      )}

      <Modal
        open={modalOpen}
        title={editingDetail ? 'Edit workflow task' : 'Create workflow task'}
        onClose={() => setModalOpen(false)}
        width={640}
      >
        <form onSubmit={handleTaskSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          <Field label="Title">
            <input autoFocus value={form.title} onChange={(event) => setForm((current) => ({ ...current, title: event.target.value }))} />
          </Field>
          <Field label="Description">
            <textarea
              value={form.description}
              onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
              rows={5}
              style={{ resize: 'vertical', minHeight: '8rem' }}
            />
          </Field>
          <Field label="Source Git URL / Path">
            <input
              value={form.source}
              onChange={(event) => setForm((current) => ({ ...current, source: event.target.value }))}
              placeholder="https://github.com/org/repo.git or /path/to/repo"
            />
          </Field>
          <Field label="Source Branch">
            <input
              value={form.source_branch}
              onChange={(event) => setForm((current) => ({ ...current, source_branch: event.target.value }))}
              placeholder="main"
            />
          </Field>
          <Field label="Owner">
            <select value={form.assignee} onChange={(event) => setForm((current) => ({ ...current, assignee: event.target.value }))}>
              <option value="">Unassigned</option>
              {employees.map((employee) => (
                <option key={employee.id} value={employee.name}>
                  {employee.name}
                </option>
              ))}
            </select>
          </Field>

          {error && (
            <div style={{ padding: '0.75rem', borderRadius: 'var(--radius-md)', background: 'rgba(239,68,68,0.08)', color: 'rgb(185, 28, 28)' }}>
              {error}
            </div>
          )}

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.6rem' }}>
            <button type="button" onClick={() => setModalOpen(false)} style={{ background: 'transparent', border: '1px solid var(--border-color)' }}>
              Cancel
            </button>
            <button type="submit" disabled={saving} style={{ background: 'var(--accent-primary)', color: 'white', border: 'none', minHeight: 44 }}>
              {saving ? 'Saving…' : editingDetail ? 'Save changes' : 'Create task'}
            </button>
          </div>
        </form>
      </Modal>

      <RejectModal
        open={Boolean(rejectPayload)}
        payload={rejectPayload}
        onClose={() => setRejectPayload(null)}
        onSubmit={(payload) => void handleTransition(payload)}
      />

      <Modal
        open={assignModalOpen}
        title={selectedTask ? `Assign ${selectedTask.task.task_id}` : 'Assign to Agent'}
        onClose={() => setAssignModalOpen(false)}
        width={480}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
          {!selectedTask ? (
            <div style={{ color: 'var(--text-tertiary)' }}>Choose a task first.</div>
          ) : !requiredRole ? (
            <div style={{ color: 'var(--text-tertiary)' }}>
              {selectedTask.task.status === 'NeedsHuman'
                ? 'This stage needs a human decision, not an agent assignment.'
                : 'Done tasks cannot be assigned to an agent.'}
            </div>
          ) : assignableEmployees.length === 0 ? (
            <div style={{ color: 'var(--text-tertiary)' }}>
              No {requiredRole} agents are available. Add one in the Company view first.
            </div>
          ) : employees.length === 0 ? (
            <div style={{ color: 'var(--text-tertiary)' }}>No agents available. Create one in the Company view first.</div>
          ) : (
            assignableEmployees.map((employee) => {
              const busy = employee.status !== 'idle'
              const unavailable = !employee.backend_available
              const disabled = busy || unavailable || assigning
              return (
                <button
                  key={employee.id}
                  type="button"
                  disabled={disabled}
                  onClick={() => void handleAssign(employee.id)}
                  style={{
                    textAlign: 'left',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.25rem',
                    padding: '0.75rem 1rem',
                    border: '1px solid var(--border-color)',
                    borderRadius: 'var(--radius-md)',
                    background: disabled ? 'var(--bg-secondary)' : 'var(--bg-primary)',
                    cursor: disabled ? 'not-allowed' : 'pointer',
                    opacity: disabled ? 0.6 : 1,
                  }}
                >
                  <div style={{ fontWeight: 600 }}>
                    {employee.name} <span style={{ color: 'var(--text-tertiary)', fontWeight: 400 }}>· {employee.role}</span>
                  </div>
                  <div style={{ fontSize: '0.8rem', color: 'var(--text-tertiary)' }}>
                    {employee.workflow_role} · {employee.agent_backend}
                    {busy && ' · busy'}
                    {unavailable && ' · backend unavailable'}
                  </div>
                </button>
              )
            })
          )}

          {assignError && (
            <div style={{ padding: '0.75rem', borderRadius: 'var(--radius-md)', background: 'rgba(239,68,68,0.08)', color: 'rgb(185, 28, 28)' }}>
              {assignError}
            </div>
          )}
        </div>
      </Modal>
    </div>
  )
}

function MetricCard({ label, value, helper, accent }: { label: string; value: string; helper: string; accent?: string }) {
  return (
    <div
      style={{
        padding: '1rem',
        borderRadius: 'var(--radius-xl)',
        background: 'var(--bg-primary)',
        border: '1px solid var(--border-color)',
      }}
    >
      <div style={{ fontSize: '0.78rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>{label}</div>
      <div style={{ marginTop: '0.35rem', fontSize: '1.6rem', fontWeight: 750, color: accent ?? 'var(--text-primary)' }}>{value}</div>
      <div style={{ marginTop: '0.25rem', fontSize: '0.82rem', color: 'var(--text-secondary)' }}>{helper}</div>
    </div>
  )
}

function TaskDetailShell({
  detail,
  employees,
  actorKey,
  onActorKeyChange,
  onEdit,
  onDelete,
  onClose,
  onTransition,
  onReject,
  onAssign,
  onPublishBranch,
  publishing,
  compact = false,
}: {
  detail: TaskDetail | null
  employees: Employee[]
  actorKey: string
  onActorKeyChange: (value: string) => void
  onEdit: () => void
  onDelete: () => void
  onClose: () => void
  onTransition: (payload: TransitionPayload) => void
  onReject: (payload: TransitionPayload) => void
  onAssign: () => void
  onPublishBranch: () => void
  publishing: boolean
  compact?: boolean
}) {
  if (!detail) {
    return (
      <div
        style={{
          minHeight: compact ? 'auto' : 540,
          padding: '1.2rem',
          borderRadius: 'var(--radius-xl)',
          background: 'var(--bg-primary)',
          border: '1px dashed var(--border-color)',
          color: 'var(--text-tertiary)',
          display: 'grid',
          placeItems: 'center',
          textAlign: 'center',
        }}
      >
        Pick a task to inspect the timeline and run the next transition.
      </div>
    )
  }

  return (
    <aside
      aria-label={`${detail.task.title} detail`}
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        gap: '1rem',
        padding: '1rem',
        borderRadius: 'var(--radius-xl)',
        background: 'var(--bg-primary)',
        border: '1px solid var(--border-color)',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '0.75rem', alignItems: 'flex-start' }}>
        <div>
          <div style={{ fontSize: '0.76rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>
            {detail.task.task_id}
          </div>
          <div style={{ marginTop: '0.35rem', fontSize: '1.15rem', fontWeight: 700 }}>{detail.task.title}</div>
        </div>
        <div style={{ display: 'flex', gap: '0.45rem', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
          <button
            type="button"
            onClick={onAssign}
            style={{ background: 'var(--accent-primary)', color: 'white', border: 'none', minHeight: 44 }}
          >
            Assign to Agent
          </button>
          <button
            type="button"
            onClick={onPublishBranch}
            disabled={publishing}
            style={{ background: 'transparent', color: 'var(--text-primary)', border: '1px solid var(--border-color)', minHeight: 44 }}
          >
            {publishing ? 'Pushing…' : 'Push Branch'}
          </button>
          <button
            type="button"
            onClick={onEdit}
            style={{ background: 'transparent', color: 'var(--text-primary)', border: '1px solid var(--border-color)', minHeight: 44 }}
          >
            Edit
          </button>
          <button type="button" onClick={onDelete} style={{ background: 'transparent', color: 'rgb(185, 28, 28)', border: '1px solid rgba(239, 68, 68, 0.2)', minHeight: 44 }}>
            Delete
          </button>
          {compact && (
            <button
              type="button"
              onClick={onClose}
              style={{ background: 'transparent', color: 'var(--text-primary)', border: '1px solid var(--border-color)', minHeight: 44 }}
            >
              Close
            </button>
          )}
        </div>
      </div>

      <div style={{ fontSize: '0.92rem', color: 'var(--text-secondary)', whiteSpace: 'pre-wrap' }}>
        {detail.task.description || 'No description provided.'}
      </div>

      <div style={{ fontSize: '0.84rem', color: 'var(--text-tertiary)', wordBreak: 'break-all' }}>
        Source: {detail.task.source || 'Missing source'}
      </div>
      <div style={{ fontSize: '0.84rem', color: 'var(--text-tertiary)', wordBreak: 'break-all' }}>
        Source branch: {detail.task.source_branch || 'main'}
      </div>
      <div style={{ fontSize: '0.84rem', color: 'var(--text-tertiary)', wordBreak: 'break-all' }}>
        Branch: {detail.task.branch_name || 'Missing branch'}
      </div>

      <TransitionActions
        detail={detail}
        employees={employees}
        actorKey={actorKey}
        onActorKeyChange={onActorKeyChange}
        onSubmit={onTransition}
        onReject={onReject}
      />

      <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: '1rem', minHeight: 0, overflowY: 'auto' }}>
        <TaskDetailTimeline detail={detail} />
      </div>
    </aside>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem' }}>
      <span style={{ fontSize: '0.76rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>
        {label}
      </span>
      {children}
    </label>
  )
}
