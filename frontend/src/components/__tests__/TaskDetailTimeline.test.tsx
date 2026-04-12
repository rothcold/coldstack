import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import TaskDetailTimeline from '../TaskDetailTimeline'

describe('TaskDetailTimeline', () => {
  it('renders empty timeline state', () => {
    render(
      <TaskDetailTimeline
        detail={{
          task: {
            id: 1,
            task_id: 'T-200',
            title: 'Fresh task',
            description: '',
            archived: false,
            status: 'Plan',
            assignee: null,
            created_at: '2026-04-09T00:00:00Z',
            subtasks: [],
          },
          events: [],
          current_action_label: 'Ready for planning',
          current_action_hint: 'Assign a planner to move the task forward.',
        }}
      />,
    )

    expect(screen.getByText('Fresh task')).toBeInTheDocument()
    expect(screen.getByText('No workflow events yet. Create the first handoff from the actions below.')).toBeInTheDocument()
    expect(screen.getByText('Assign a planner to move the task forward.')).toBeInTheDocument()
  })

  it('renders event sentences and evidence text', () => {
    render(
      <TaskDetailTimeline
        detail={{
          task: {
            id: 2,
            task_id: 'T-201',
            title: 'Returned task',
            description: '',
            archived: false,
            status: 'Coding',
            assignee: 'Alice',
            created_at: '2026-04-09T00:00:00Z',
            subtasks: [],
          },
          events: [
            {
              id: 10,
              task_id: 2,
              from_status: 'QA',
              to_status: 'Coding',
              actor_type: 'employee',
              actor_id: 99,
              actor_label: 'QA Bot',
              action: 'reject',
              note: 'Test case is still failing.',
              evidence_text: 'stack trace',
              created_at: '2026-04-09T01:00:00Z',
            },
          ],
          current_action_label: 'Returned to coding',
          current_action_hint: null,
        }}
      />,
    )

    expect(screen.getByText('QA Bot returned this task to Coding')).toBeInTheDocument()
    expect(screen.getByText('Test case is still failing.')).toBeInTheDocument()
    expect(screen.getByText('stack trace')).toBeInTheDocument()
  })
})
