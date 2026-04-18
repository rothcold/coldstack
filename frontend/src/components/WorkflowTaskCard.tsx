import type { BoardTaskSummary } from '../types'

interface WorkflowTaskCardProps {
  task: BoardTaskSummary
  selected: boolean
  onClick: () => void
}

function statusTone(task: BoardTaskSummary) {
  if (task.waiting_for_human) {
    return {
      border: 'var(--card-waiting-border)',
      background: 'var(--card-waiting-bg)',
      badgeBg: 'var(--card-waiting-badge-bg)',
      badgeColor: 'var(--card-waiting-badge-color)',
      label: 'Needs your decision',
    }
  }
  if (task.waiting_for_agent) {
    return {
      border: 'rgba(8, 145, 178, 0.3)',
      background: 'rgba(8, 145, 178, 0.07)',
      badgeBg: 'rgba(8, 145, 178, 0.14)',
      badgeColor: 'rgb(14, 116, 144)',
      label: 'Waiting for agent',
    }
  }
  if (task.needs_attention) {
    return {
      border: 'var(--card-returned-border)',
      background: 'var(--card-returned-bg)',
      badgeBg: 'var(--card-returned-badge-bg)',
      badgeColor: 'var(--card-returned-badge-color)',
      label: 'Returned',
    }
  }
  return {
    border: 'var(--border-color)',
    background: 'var(--bg-primary)',
    badgeBg: 'var(--bg-tertiary)',
    badgeColor: 'var(--text-secondary)',
    label: task.status,
  }
}

export default function WorkflowTaskCard({ task, selected, onClick }: WorkflowTaskCardProps) {
  const tone = statusTone(task)

  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        width: '100%',
        textAlign: 'left',
        background: tone.background,
        border: `1px solid ${selected ? 'var(--accent-primary)' : tone.border}`,
        boxShadow: selected ? '0 0 0 2px var(--accent-light)' : 'none',
        padding: '0.95rem',
        borderRadius: 'var(--radius-lg)',
        display: 'flex',
        flexDirection: 'column',
        gap: '0.7rem',
        minHeight: 152,
      }}
      aria-pressed={selected}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '0.75rem' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '0.76rem', color: 'var(--text-tertiary)', letterSpacing: '0.08em', textTransform: 'uppercase' }}>
            {task.task_id}
          </div>
          <div style={{ marginTop: '0.35rem', fontWeight: 650, fontSize: '1rem', color: 'var(--text-primary)' }}>
            {task.title}
          </div>
        </div>
        <span
          style={{
            flexShrink: 0,
            padding: '0.24rem 0.55rem',
            borderRadius: '999px',
            background: tone.badgeBg,
            color: tone.badgeColor,
            fontSize: '0.7rem',
            fontWeight: 700,
            letterSpacing: '0.04em',
            textTransform: 'uppercase',
          }}
        >
          {tone.label}
        </span>
      </div>

      <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
        <span
          style={{
            padding: '0.25rem 0.5rem',
            borderRadius: 'var(--radius-sm)',
            background: 'var(--card-chip-bg)',
            color: 'var(--text-secondary)',
            fontSize: '0.76rem',
          }}
        >
          {task.status}
        </span>
        {task.assignee && (
          <span
            style={{
              padding: '0.25rem 0.5rem',
              borderRadius: 'var(--radius-sm)',
              background: 'var(--card-chip-bg)',
              color: 'var(--text-secondary)',
              fontSize: '0.76rem',
            }}
          >
            {task.assignee}
          </span>
        )}
      </div>

      <div style={{ marginTop: 'auto', fontSize: '0.82rem', color: 'var(--text-secondary)', lineHeight: 1.45 }}>
        {task.latest_event_summary || 'No workflow events yet.'}
      </div>
    </button>
  )
}
