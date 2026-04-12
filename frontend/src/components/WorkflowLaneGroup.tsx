import type { BoardTaskSummary, WorkflowBoardGroup } from '../types'
import WorkflowTaskCard from './WorkflowTaskCard'

interface WorkflowLaneGroupProps {
  group: WorkflowBoardGroup
  title: string
  description: string
  tasks: BoardTaskSummary[]
  selectedTaskId: number | null
  onSelectTask: (taskId: number) => void
}

export default function WorkflowLaneGroup({
  group,
  title,
  description,
  tasks,
  selectedTaskId,
  onSelectTask,
}: WorkflowLaneGroupProps) {
  return (
    <section
      aria-label={`${title} lane`}
      style={{
        minWidth: 280,
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        gap: '0.85rem',
        padding: '1rem',
        borderRadius: 'var(--radius-xl)',
        background: 'var(--lane-bg)',
        border: '1px solid var(--border-color)',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '0.75rem', alignItems: 'baseline' }}>
        <div>
          <div style={{ fontSize: '0.85rem', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase' }}>
            {title}
          </div>
          <div style={{ marginTop: '0.2rem', fontSize: '0.8rem', color: 'var(--text-tertiary)' }}>
            {description}
          </div>
        </div>
        <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)' }}>{tasks.length}</div>
      </div>

      {tasks.length === 0 ? (
        <div
          style={{
            minHeight: 120,
            border: '1px dashed var(--border-color)',
            borderRadius: 'var(--radius-lg)',
            display: 'grid',
            placeItems: 'center',
            color: 'var(--text-tertiary)',
            fontSize: '0.82rem',
            padding: '1rem',
            textAlign: 'center',
          }}
        >
          No tasks in {group.toLowerCase()}.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
          {tasks.map((task) => (
            <WorkflowTaskCard
              key={task.id}
              task={task}
              selected={selectedTaskId === task.id}
              onClick={() => onSelectTask(task.id)}
            />
          ))}
        </div>
      )}
    </section>
  )
}
