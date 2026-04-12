import type {
  Employee,
  TaskDetail,
  TransitionPayload,
  WorkflowActorType,
  WorkflowRole,
  WorkflowStatus,
} from '../types'

interface ActorOption {
  actorType: WorkflowActorType
  actorId?: number
  actorLabel: string
  workflowRole: WorkflowRole
}

interface TransitionActionsProps {
  detail: TaskDetail
  employees: Employee[]
  actorKey: string
  onActorKeyChange: (value: string) => void
  onSubmit: (payload: TransitionPayload) => void
  onReject: (payload: TransitionPayload) => void
}

function nextAdvance(status: WorkflowStatus): WorkflowStatus | null {
  switch (status) {
    case 'Plan':
      return 'Design'
    case 'Design':
      return 'Coding'
    case 'Coding':
      return 'Review'
    case 'Review':
      return 'QA'
    case 'QA':
      return 'NeedsHuman'
    case 'NeedsHuman':
      return 'Done'
    default:
      return null
  }
}

function allowsAdvance(role: WorkflowRole, actorType: WorkflowActorType, status: WorkflowStatus) {
  if (actorType === 'human') {
    return role === 'human' && status === 'NeedsHuman'
  }
  return (
    (role === 'planner' && status === 'Plan') ||
    (role === 'designer' && status === 'Design') ||
    (role === 'coder' && status === 'Coding') ||
    (role === 'reviewer' && status === 'Review') ||
    (role === 'qa' && status === 'QA')
  )
}

function allowsReject(role: WorkflowRole, actorType: WorkflowActorType, status: WorkflowStatus) {
  if (actorType === 'human') {
    return role === 'human' && status === 'NeedsHuman'
  }
  return (role === 'reviewer' && status === 'Review') || (role === 'qa' && status === 'QA')
}

function allowsArchive(role: WorkflowRole, actorType: WorkflowActorType, status: WorkflowStatus) {
  return actorType === 'human' && role === 'human' && status === 'Done'
}

export default function TransitionActions({
  detail,
  employees,
  actorKey,
  onActorKeyChange,
  onSubmit,
  onReject,
}: TransitionActionsProps) {
  const actorOptions: ActorOption[] = [
    ...employees.map((employee) => ({
      actorType: 'employee' as const,
      actorId: employee.id,
      actorLabel: employee.name,
      workflowRole: employee.workflow_role,
    })),
    {
      actorType: 'human' as const,
      actorLabel: 'Human decision',
      workflowRole: 'human',
    },
  ]

  const selectedActor =
    actorOptions.find((option) => `${option.actorType}:${option.actorId ?? 'self'}` === actorKey) ??
    actorOptions[0]

  const basePayload = {
    actor_type: selectedActor.actorType,
    actor_id: selectedActor.actorId ?? null,
    actor_label: selectedActor.actorLabel,
    from_status: detail.task.status,
  } satisfies Pick<TransitionPayload, 'actor_type' | 'actor_id' | 'actor_label' | 'from_status'>

  const advanceTo = nextAdvance(detail.task.status)
  const canAdvance = advanceTo ? allowsAdvance(selectedActor.workflowRole, selectedActor.actorType, detail.task.status) : false
  const canReject = allowsReject(selectedActor.workflowRole, selectedActor.actorType, detail.task.status)
  const canArchive = allowsArchive(selectedActor.workflowRole, selectedActor.actorType, detail.task.status)

  const buttonStyle: React.CSSProperties = {
    minHeight: 44,
    padding: '0.7rem 0.95rem',
    fontWeight: 600,
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.85rem' }}>
      <div>
        <div style={{ fontSize: '0.78rem', color: 'var(--text-tertiary)', textTransform: 'uppercase', letterSpacing: '0.08em' }}>
          Acting as
        </div>
        <select
          value={actorKey}
          onChange={(event) => onActorKeyChange(event.target.value)}
          style={{ width: '100%', marginTop: '0.4rem', minHeight: 44 }}
        >
          {actorOptions.map((option) => {
            const key = `${option.actorType}:${option.actorId ?? 'self'}`
            return (
              <option key={key} value={key}>
                {option.actorLabel} · {option.workflowRole}
              </option>
            )
          })}
        </select>
      </div>

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.6rem' }}>
        {canAdvance && advanceTo && (
          <button
            type="button"
            onClick={() =>
              onSubmit({
                ...basePayload,
                action: 'advance',
                to_status: advanceTo,
              })
            }
            style={{
              ...buttonStyle,
              background: 'var(--accent-primary)',
              color: 'white',
              border: 'none',
            }}
          >
            {detail.task.status === 'NeedsHuman' ? 'Approve' : `Advance to ${advanceTo}`}
          </button>
        )}

        {canReject && (
          <button
            type="button"
            onClick={() =>
              onReject({
                ...basePayload,
                action: 'reject',
                to_status: 'Coding',
              })
            }
            style={{
              ...buttonStyle,
              background: 'rgba(239, 68, 68, 0.08)',
              color: 'rgb(185, 28, 28)',
              border: '1px solid rgba(239, 68, 68, 0.2)',
            }}
          >
            Return to Coding
          </button>
        )}

        {canArchive && (
          <button
            type="button"
            onClick={() =>
              onSubmit({
                ...basePayload,
                action: 'archive',
                to_status: 'Done',
              })
            }
            style={{
              ...buttonStyle,
              background: 'rgba(180, 83, 9, 0.12)',
              color: 'rgb(180, 83, 9)',
              border: '1px solid rgba(180, 83, 9, 0.24)',
            }}
          >
            Approve and archive
          </button>
        )}
      </div>
    </div>
  )
}
