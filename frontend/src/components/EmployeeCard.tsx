import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CurrentExecution, Employee } from '../types'
import { subscribeExecution } from '../lib/executionStream'

interface EmployeeCardProps {
  employee: Employee
  onClick: () => void
  selected: boolean
}

const statusMeta = (status: string) => {
  switch (status) {
    case 'working':
      return { dot: 'var(--status-working)', label: 'Working', bg: 'var(--status-working-bg)', color: 'var(--status-working)' }
    case 'error':
      return { dot: 'var(--status-offline)', label: 'Error', bg: 'var(--status-offline-bg)', color: 'var(--status-offline)' }
    default:
      return { dot: 'var(--status-idle)', label: 'Idle', bg: 'var(--status-idle-bg)', color: 'var(--status-idle)' }
  }
}

export default function EmployeeCard({ employee, onClick, selected }: EmployeeCardProps) {
  const s = statusMeta(employee.status)
  const [currentExecution, setCurrentExecution] = useState<CurrentExecution | null>(null)
  const [tail, setTail] = useState<string[]>([])
  const [runtimeSeconds, setRuntimeSeconds] = useState(0)
  const [streamError, setStreamError] = useState<string | null>(null)

  const fetchCurrentExecution = useCallback(async () => {
    try {
      const response = await fetch(`/api/employees/${employee.id}/current_execution`)
      if (response.status === 404) {
        setCurrentExecution(null)
        setTail([])
        setRuntimeSeconds(0)
        return
      }
      if (!response.ok) {
        throw new Error(await response.text())
      }
      const execution = (await response.json()) as CurrentExecution
      setCurrentExecution(execution)
      setRuntimeSeconds(Math.max(0, Math.floor((Date.now() - new Date(execution.started_at).getTime()) / 1000)))
    } catch (error) {
      setStreamError(error instanceof Error ? error.message : 'Failed to load current execution')
    }
  }, [employee.id])

  useEffect(() => {
    void fetchCurrentExecution()
    const timer = window.setInterval(() => void fetchCurrentExecution(), 5000)
    return () => window.clearInterval(timer)
  }, [fetchCurrentExecution])

  useEffect(() => {
    if (!currentExecution) return
    return subscribeExecution(currentExecution.execution_id, {
      onOutput: (event) => {
        const lines = event.chunk.split('\n').map((line) => line.trim()).filter(Boolean)
        if (lines.length === 0) return
        setTail((prev) => [...prev, ...lines].slice(-3))
      },
      onStatus: () => {
        void fetchCurrentExecution()
      },
      onError: () => {
        setStreamError('Execution stream connection failed')
      },
    })
  }, [currentExecution, fetchCurrentExecution])

  useEffect(() => {
    if (!currentExecution) return
    const timer = window.setInterval(() => {
      setRuntimeSeconds(Math.max(0, Math.floor((Date.now() - new Date(currentExecution.started_at).getTime()) / 1000)))
    }, 1000)
    return () => window.clearInterval(timer)
  }, [currentExecution])

  const runtimeLabel = useMemo(() => {
    const min = Math.floor(runtimeSeconds / 60)
    const sec = runtimeSeconds % 60
    return `${min}:${sec.toString().padStart(2, '0')}`
  }, [runtimeSeconds])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onClick()
    }
  }

  return (
    <div
      onClick={onClick}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      role="button"
      aria-pressed={selected}
      aria-label={`${employee.name}, ${employee.role}, workflow role: ${employee.workflow_role}, status: ${employee.status}, backend ${employee.backend_available ? 'available' : 'unavailable'}`}
      style={{
        padding: '1rem 1.1rem',
        borderRadius: 'var(--radius-lg)',
        background: 'var(--bg-primary)',
        border: `1px solid ${selected ? 'var(--accent-primary)' : 'var(--border-color)'}`,
        boxShadow: selected
          ? '0 0 0 3px var(--accent-light)'
          : '0 1px 2px rgba(15, 23, 42, 0.04)',
        cursor: 'pointer',
        display: 'flex',
        flexDirection: 'column',
        gap: '0.75rem',
        outline: 'none',
        transition: 'border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease',
      }}
      onMouseEnter={(e) => {
        if (!selected) e.currentTarget.style.transform = 'translateY(-1px)'
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.transform = 'translateY(0)'
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '0.5rem' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, fontSize: '1rem', color: 'var(--text-primary)', letterSpacing: '-0.005em' }}>
            {employee.name}
          </div>
          <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginTop: '0.15rem' }}>
            {employee.role}
          </div>
        </div>
        <span
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.35rem',
            padding: '0.2rem 0.55rem',
            borderRadius: 'var(--radius-full)',
            background: s.bg,
            color: s.color,
            fontSize: '0.68rem',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.04em',
          }}
        >
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: s.dot }} aria-hidden="true" />
          {s.label}
        </span>
      </div>
      <div style={{ display: 'flex', gap: '0.4rem', flexWrap: 'wrap', fontSize: '0.7rem' }}>
        <span
          style={{
            padding: '0.2rem 0.55rem',
            background: 'var(--bg-tertiary)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-secondary)',
          }}
        >
          {employee.department}
        </span>
        <span
          style={{
            padding: '0.2rem 0.55rem',
            background: 'rgba(15, 23, 42, 0.06)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-secondary)',
            textTransform: 'uppercase',
          }}
        >
          {employee.workflow_role}
        </span>
        <span
          style={{
            padding: '0.2rem 0.55rem',
            background: 'var(--bg-tertiary)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-secondary)',
            fontFamily: 'var(--font-mono)',
          }}
        >
          {employee.agent_backend}
        </span>
        {!employee.backend_available && (
          <span
            style={{
              padding: '0.2rem 0.55rem',
              background: 'var(--status-offline-bg)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--status-offline)',
              textTransform: 'uppercase',
            }}
          >
            backend unavailable
          </span>
        )}
      </div>

      {currentExecution && (
        <div
          style={{
            border: '1px solid var(--border-color)',
            borderRadius: 'var(--radius-md)',
            padding: '0.55rem',
            background: 'var(--bg-secondary)',
            display: 'flex',
            flexDirection: 'column',
            gap: '0.35rem',
          }}
        >
          <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)', display: 'flex', justifyContent: 'space-between', gap: '0.5rem' }}>
            <span>{currentExecution.task_key} · {currentExecution.task_title}</span>
            <span>{runtimeLabel}</span>
          </div>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: '0.72rem', color: 'var(--text-primary)', minHeight: '3.1em' }}>
            {tail.length ? tail.map((line, index) => <div key={`${line}-${index}`}>{line}</div>) : <div>Waiting for output...</div>}
          </div>
        </div>
      )}

      {streamError && (
        <div style={{ fontSize: '0.72rem', color: 'var(--status-offline)' }}>
          {streamError}
        </div>
      )}
    </div>
  )
}
