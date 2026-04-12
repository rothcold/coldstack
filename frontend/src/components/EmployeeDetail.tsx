import { useEffect, useState } from 'react'
import type { Employee, Execution } from '../types'
import LiveTerminal from './LiveTerminal'

interface EmployeeDetailProps {
  employee: Employee
  onClose: () => void
  onEdit: () => void
  onDelete: () => void
}

const statusPillColor = (status: string): React.CSSProperties => {
  switch (status) {
    case 'working':
      return { background: 'var(--status-working-bg)', color: 'var(--status-working)' }
    case 'error':
      return { background: 'var(--status-offline-bg)', color: 'var(--status-offline)' }
    default:
      return { background: 'var(--status-idle-bg)', color: 'var(--status-idle)' }
  }
}

export default function EmployeeDetail({ employee, onClose, onEdit, onDelete }: EmployeeDetailProps) {
  const [executions, setExecutions] = useState<Execution[]>([])

  useEffect(() => {
    fetch(`/api/employees/${employee.id}/executions`)
      .then(res => res.json())
      .then(data => setExecutions(data))
      .catch(err => console.error('Failed to fetch executions:', err))
  }, [employee.id])

  const runningExecution = executions.find(e => e.status === 'running')
  const latestExecution = executions[0]

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  return (
    <div
      style={{
        background: 'var(--bg-primary)',
        borderRadius: 'var(--radius-lg)',
        border: '1px solid var(--border-color)',
        padding: '1.5rem',
        display: 'flex',
        flexDirection: 'column',
        gap: '1.25rem',
        height: '100%',
        overflowY: 'auto',
      }}
      role="region"
      aria-label={`Details for ${employee.name}`}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '1rem' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', flexWrap: 'wrap' }}>
            <h2 style={{ fontSize: '1.5rem', fontWeight: 700, letterSpacing: '-0.01em' }}>{employee.name}</h2>
            <span
              style={{
                ...statusPillColor(employee.status),
                padding: '0.2rem 0.55rem',
                borderRadius: 'var(--radius-full)',
                fontSize: '0.7rem',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              {employee.status}
            </span>
          </div>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginTop: '0.25rem' }}>
            {employee.role} · {employee.department} · <code style={{ fontSize: '0.8rem' }}>{employee.agent_backend}</code>
          </div>
          <div style={{ color: 'var(--text-tertiary)', fontSize: '0.8rem', marginTop: '0.2rem', textTransform: 'uppercase', letterSpacing: '0.06em' }}>
            Workflow role: {employee.workflow_role}
          </div>
        </div>
        <div style={{ display: 'flex', gap: '0.4rem', flexShrink: 0 }}>
          <button
            onClick={onEdit}
            style={{
              background: 'var(--bg-secondary)',
              border: '1px solid var(--border-color)',
              color: 'var(--text-primary)',
              padding: '0.4rem 0.75rem',
              fontSize: '0.8rem',
            }}
          >
            Edit
          </button>
          <button
            onClick={onDelete}
            style={{
              background: 'transparent',
              border: '1px solid var(--border-color)',
              color: 'var(--status-offline)',
              padding: '0.4rem 0.75rem',
              fontSize: '0.8rem',
            }}
          >
            Delete
          </button>
          <button
            onClick={onClose}
            aria-label="Close detail panel"
            style={{
              background: 'transparent',
              border: '1px solid var(--border-color)',
              color: 'var(--text-secondary)',
              padding: '0.4rem 0.65rem',
              fontSize: '0.8rem',
            }}
          >
            Esc
          </button>
        </div>
      </div>

      <Section title="System Prompt">
        <div
          style={{
            background: 'var(--bg-secondary)',
            padding: '1rem',
            borderRadius: 'var(--radius-md)',
            fontSize: '0.85rem',
            color: 'var(--text-secondary)',
            whiteSpace: 'pre-wrap',
            border: '1px solid var(--border-color)',
          }}
        >
          {employee.system_prompt || <em style={{ color: 'var(--text-tertiary)' }}>No system prompt defined.</em>}
        </div>
      </Section>

      <Section title="Live Terminal">
        {!employee.backend_available && (
          <div
            style={{
              marginBottom: '0.75rem',
              padding: '0.75rem',
              background: 'var(--status-offline-bg)',
              color: 'var(--status-offline)',
              borderRadius: 'var(--radius-md)',
              border: '1px solid var(--status-offline)',
              fontSize: '0.82rem',
            }}
          >
            Backend <code>{employee.agent_backend}</code> is not available on this server. Check CLI installation/path before assigning tasks.
          </div>
        )}
        {runningExecution ? (
          <LiveTerminal executionId={runningExecution.id} />
        ) : latestExecution ? (
          <LiveTerminal executionId={latestExecution.id} />
        ) : (
          <div
            style={{
              padding: '1.5rem',
              background: 'var(--bg-secondary)',
              borderRadius: 'var(--radius-md)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              border: '1px dashed var(--border-color)',
              fontSize: '0.85rem',
            }}
          >
            No recent activity.
          </div>
        )}
      </Section>

      <Section title="Execution History">
        {executions.length === 0 ? (
          <div style={{ color: 'var(--text-tertiary)', fontSize: '0.85rem' }}>No executions yet.</div>
        ) : (
          <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
            {executions.map(exec => (
              <li
                key={exec.id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '0.6rem 0.85rem',
                  background: 'var(--bg-secondary)',
                  borderRadius: 'var(--radius-md)',
                  fontSize: '0.85rem',
                  border: '1px solid var(--border-color)',
                }}
              >
                <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
                  <span style={{ fontWeight: 500 }}>Task #{exec.task_id}</span>
                  <span style={{ color: 'var(--text-tertiary)', fontSize: '0.75rem' }}>
                    {new Date(exec.started_at).toLocaleString()}
                  </span>
                </div>
                <span
                  style={{
                    fontSize: '0.7rem',
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    color:
                      exec.status === 'completed'
                        ? 'var(--status-done)'
                        : exec.status === 'failed'
                        ? 'var(--status-offline)'
                        : exec.status === 'running'
                        ? 'var(--status-working)'
                        : 'var(--text-tertiary)',
                  }}
                >
                  {exec.status}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Section>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3
        style={{
          fontSize: '0.7rem',
          fontWeight: 700,
          textTransform: 'uppercase',
          letterSpacing: '0.06em',
          color: 'var(--text-tertiary)',
          marginBottom: '0.6rem',
        }}
      >
        {title}
      </h3>
      {children}
    </div>
  )
}
