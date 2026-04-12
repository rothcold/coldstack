import type { BoardTaskSummary, WorkflowBoardGroup } from '../types'
import WorkflowLaneGroup from './WorkflowLaneGroup'

interface WorkflowBoardProps {
  groups: Record<WorkflowBoardGroup, BoardTaskSummary[]>
  selectedTaskId: number | null
  onSelectTask: (taskId: number) => void
  compact: boolean
  activeGroup: WorkflowBoardGroup
  onActiveGroupChange: (group: WorkflowBoardGroup) => void
}

const GROUP_META: Record<WorkflowBoardGroup, { title: string; description: string }> = {
  Plan: { title: 'Plan', description: 'Scope and frame the task.' },
  Build: { title: 'Build', description: 'Design and coding in flight.' },
  Review: { title: 'Review', description: 'Peer gate before QA.' },
  QA: { title: 'QA', description: 'Validation before handoff.' },
  Human: { title: 'Human', description: 'Needs your decision.' },
  Done: { title: 'Done', description: 'Ready to archive.' },
}

const GROUP_ORDER: WorkflowBoardGroup[] = ['Plan', 'Build', 'Review', 'QA', 'Human', 'Done']

export default function WorkflowBoard({
  groups,
  selectedTaskId,
  onSelectTask,
  compact,
  activeGroup,
  onActiveGroupChange,
}: WorkflowBoardProps) {
  if (compact) {
    const meta = GROUP_META[activeGroup]
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <div style={{ display: 'flex', gap: '0.5rem', overflowX: 'auto', paddingBottom: '0.25rem' }}>
          {GROUP_ORDER.map((group) => (
            <button
              key={group}
              type="button"
              onClick={() => onActiveGroupChange(group)}
              aria-pressed={activeGroup === group}
              style={{
                background: activeGroup === group ? 'var(--accent-primary)' : 'var(--bg-primary)',
                color: activeGroup === group ? 'white' : 'var(--text-secondary)',
                border: `1px solid ${activeGroup === group ? 'var(--accent-primary)' : 'var(--border-color)'}`,
                minWidth: 104,
                minHeight: 44,
              }}
            >
              {meta.title === GROUP_META[group].title ? `• ${GROUP_META[group].title}` : GROUP_META[group].title}
            </button>
          ))}
        </div>

        <WorkflowLaneGroup
          group={activeGroup}
          title={meta.title}
          description={meta.description}
          tasks={groups[activeGroup]}
          selectedTaskId={selectedTaskId}
          onSelectTask={onSelectTask}
        />
      </div>
    )
  }

  return (
    <div style={{ display: 'flex', gap: '1rem', overflowX: 'auto', alignItems: 'stretch', paddingBottom: '0.5rem' }}>
      {GROUP_ORDER.map((group) => (
        <WorkflowLaneGroup
          key={group}
          group={group}
          title={GROUP_META[group].title}
          description={GROUP_META[group].description}
          tasks={groups[group]}
          selectedTaskId={selectedTaskId}
          onSelectTask={onSelectTask}
        />
      ))}
    </div>
  )
}
