import type { TaskDetail, WorkflowEvent } from '../types'

interface TaskDetailTimelineProps {
  detail: TaskDetail
}

function eventSentence(event: WorkflowEvent) {
  if (event.action === 'reject') {
    return `${event.actor_label} returned this task to ${event.to_status}`
  }
  if (event.action === 'archive') {
    return `${event.actor_label} archived this task`
  }
  return `${event.actor_label} moved this task to ${event.to_status}`
}

export default function TaskDetailTimeline({ detail }: TaskDetailTimelineProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.45rem' }}>
        <div style={{ fontSize: '0.78rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>
          Timeline
        </div>
        <div style={{ fontSize: '1.1rem', fontWeight: 650 }}>{detail.task.title}</div>
        <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
          {detail.task.task_id} · {detail.current_action_label}
        </div>
        {detail.current_action_hint && (
          <div
            style={{
              padding: '0.75rem 0.9rem',
              borderRadius: 'var(--radius-md)',
              background: 'rgba(15, 23, 42, 0.04)',
              color: 'var(--text-secondary)',
              fontSize: '0.86rem',
            }}
          >
            {detail.current_action_hint}
          </div>
        )}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        {detail.events.length === 0 ? (
          <div
            style={{
              padding: '1rem',
              borderRadius: 'var(--radius-lg)',
              border: '1px dashed var(--border-color)',
              color: 'var(--text-tertiary)',
              fontSize: '0.86rem',
            }}
          >
            No workflow events yet. Create the first handoff from the actions below.
          </div>
        ) : (
          detail.events.map((event) => (
            <div key={event.id} style={{ display: 'grid', gridTemplateColumns: '16px 1fr', gap: '0.8rem' }}>
              <div style={{ display: 'flex', justifyContent: 'center' }}>
                <div style={{ width: 2, background: 'var(--border-color)', position: 'relative' }}>
                  <span
                    aria-hidden="true"
                    style={{
                      position: 'absolute',
                      top: 4,
                      left: -5,
                      width: 12,
                      height: 12,
                      borderRadius: '50%',
                      background: event.action === 'reject' ? 'rgb(185, 28, 28)' : event.action === 'archive' ? 'rgb(180, 83, 9)' : 'var(--accent-primary)',
                    }}
                  />
                </div>
              </div>
              <div style={{ paddingBottom: '0.35rem' }}>
                <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{eventSentence(event)}</div>
                <div style={{ marginTop: '0.2rem', fontSize: '0.78rem', color: 'var(--text-tertiary)' }}>
                  {new Date(event.created_at).toLocaleString()} · {event.from_status} → {event.to_status}
                </div>
                {event.note && (
                  <div style={{ marginTop: '0.55rem', fontSize: '0.86rem', color: 'var(--text-secondary)' }}>
                    {event.note}
                  </div>
                )}
                {event.evidence_text && (
                  <pre
                    style={{
                      marginTop: '0.55rem',
                      whiteSpace: 'pre-wrap',
                      padding: '0.8rem',
                      background: 'var(--bg-secondary)',
                      border: '1px solid var(--border-color)',
                      borderRadius: 'var(--radius-md)',
                      fontSize: '0.8rem',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {event.evidence_text}
                  </pre>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
