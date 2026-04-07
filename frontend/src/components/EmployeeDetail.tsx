import { useEffect, useState } from 'react'
import type { Employee, Execution } from '../types'
import LiveTerminal from './LiveTerminal'

interface EmployeeDetailProps {
  employee: Employee
  onClose: () => void
}

export default function EmployeeDetail({ employee, onClose }: EmployeeDetailProps) {
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
      if (e.key === 'Escape') {
        onClose()
      }
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
        gap: '1.5rem',
        height: '100%',
        overflowY: 'auto'
      }}
      role="region"
      aria-label={`Details for ${employee.name}`}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <h2 style={{ margin: '0 0 0.25rem 0', fontSize: '1.5rem' }}>{employee.name}</h2>
          <div style={{ color: 'var(--text-secondary)' }}>{employee.role} &bull; {employee.department}</div>
        </div>
        <button 
          onClick={onClose}
          aria-label="Close detail panel"
          style={{ background: 'transparent', border: '1px solid var(--border-color)', padding: '0.25rem 0.5rem', color: 'var(--text-secondary)' }}
        >
          Esc
        </button>
      </div>

      <div>
        <h3 style={{ fontSize: '1rem', margin: '0 0 0.5rem 0' }}>System Prompt</h3>
        <div style={{ background: 'var(--bg-secondary)', padding: '1rem', borderRadius: 'var(--radius-md)', fontSize: '0.875rem', color: 'var(--text-secondary)', whiteSpace: 'pre-wrap' }}>
          {employee.system_prompt || 'No system prompt defined.'}
        </div>
      </div>

      <div>
        <h3 style={{ fontSize: '1rem', margin: '0 0 0.5rem 0' }}>Live Terminal</h3>
        {runningExecution ? (
          <LiveTerminal executionId={runningExecution.id} />
        ) : latestExecution ? (
          <LiveTerminal executionId={latestExecution.id} />
        ) : (
          <div style={{ padding: '1rem', background: 'var(--bg-secondary)', borderRadius: 'var(--radius-md)', color: 'var(--text-tertiary)', textAlign: 'center' }}>
            No recent activity.
          </div>
        )}
      </div>

      <div>
        <h3 style={{ fontSize: '1rem', margin: '0 0 0.5rem 0' }}>Execution History</h3>
        {executions.length === 0 ? (
          <div style={{ color: 'var(--text-tertiary)', fontSize: '0.875rem' }}>No executions yet.</div>
        ) : (
          <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
            {executions.map(exec => (
              <li key={exec.id} style={{ display: 'flex', justifyContent: 'space-between', padding: '0.75rem', background: 'var(--bg-secondary)', borderRadius: 'var(--radius-md)', fontSize: '0.875rem' }}>
                <div style={{ display: 'flex', gap: '1rem' }}>
                  <span style={{ fontWeight: '500' }}>Task #{exec.task_id}</span>
                  <span style={{ color: 'var(--text-secondary)' }}>{new Date(exec.started_at).toLocaleString()}</span>
                </div>
                <span style={{ 
                  color: exec.status === 'completed' ? 'var(--status-done)' : 
                         exec.status === 'failed' ? 'var(--status-offline)' : 
                         exec.status === 'running' ? 'var(--status-working)' : 'var(--text-tertiary)' 
                }}>
                  {exec.status}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}
