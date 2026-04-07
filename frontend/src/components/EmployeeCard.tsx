import type { Employee } from '../types'

interface EmployeeCardProps {
  employee: Employee
  onClick: () => void
  selected: boolean
}

export default function EmployeeCard({ employee, onClick, selected }: EmployeeCardProps) {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'idle': return 'var(--status-idle)'
      case 'working': return 'var(--status-working)'
      case 'offline': return 'var(--status-offline)'
      default: return 'var(--status-idle)'
    }
  }

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
      aria-label={`${employee.name}, ${employee.role}, status: ${employee.status}`}
      style={{
        padding: '1rem',
        borderRadius: 'var(--radius-lg)',
        background: selected ? 'var(--accent-light)' : 'var(--bg-primary)',
        border: `1px solid ${selected ? 'var(--accent-primary)' : 'var(--border-color)'}`,
        cursor: 'pointer',
        display: 'flex',
        flexDirection: 'column',
        gap: '0.5rem',
        outline: 'none',
        transition: 'all 0.2s ease-in-out'
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div>
          <div style={{ fontWeight: 'bold', fontSize: '1.1rem', color: 'var(--text-primary)' }}>{employee.name}</div>
          <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{employee.role} &bull; {employee.department}</div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
          <div 
            style={{ 
              width: '8px', height: '8px', borderRadius: '50%', 
              background: getStatusColor(employee.status)
            }} 
            aria-hidden="true"
          />
          <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'capitalize' }}>
            {employee.status}
          </span>
        </div>
      </div>
      <div style={{ fontSize: '0.75rem', color: 'var(--text-tertiary)', display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
        <span style={{ padding: '0.1rem 0.4rem', background: 'var(--bg-tertiary)', borderRadius: 'var(--radius-sm)' }}>
          {employee.agent_backend}
        </span>
      </div>
    </div>
  )
}
