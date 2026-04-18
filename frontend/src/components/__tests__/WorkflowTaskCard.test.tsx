import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import WorkflowTaskCard from '../WorkflowTaskCard'

describe('WorkflowTaskCard', () => {
  it('renders waiting human priority copy', () => {
    render(
      <WorkflowTaskCard
        task={{
          id: 1,
          task_id: 'T-100',
          title: 'Human signoff',
          status: 'NeedsHuman',
          board_group: 'Human',
          assignee: 'Owner',
          archived: false,
          needs_attention: false,
          waiting_for_human: true,
          waiting_for_agent: false,
          rejection_count: 0,
          latest_event_summary: 'Needs your decision',
        }}
        selected={false}
        onClick={vi.fn()}
      />,
    )

    expect(screen.getByText('Human signoff')).toBeInTheDocument()
    expect(screen.getAllByText('Needs your decision')).toHaveLength(2)
    expect(screen.getByText('Owner')).toBeInTheDocument()
  })

  it('renders returned task summary text', () => {
    render(
      <WorkflowTaskCard
        task={{
          id: 2,
          task_id: 'T-101',
          title: 'Returned task',
          status: 'Coding',
          board_group: 'Build',
          assignee: null,
          archived: false,
          needs_attention: true,
          waiting_for_human: false,
          waiting_for_agent: false,
          rejection_count: 1,
          latest_event_summary: 'Returned by QA with notes',
        }}
        selected
        onClick={vi.fn()}
      />,
    )

    expect(screen.getByText('Returned')).toBeInTheDocument()
    expect(screen.getByText('Returned by QA with notes')).toBeInTheDocument()
  })

  it('renders waiting for next agent state', () => {
    render(
      <WorkflowTaskCard
        task={{
          id: 3,
          task_id: 'T-102',
          title: 'Waiting task',
          status: 'Review',
          board_group: 'Review',
          assignee: null,
          archived: false,
          needs_attention: false,
          waiting_for_human: false,
          waiting_for_agent: true,
          rejection_count: 0,
          latest_event_summary: 'Waiting for next idle reviewer agent',
        }}
        selected={false}
        onClick={vi.fn()}
      />,
    )

    expect(screen.getByText('Waiting for agent')).toBeInTheDocument()
    expect(screen.getByText('Waiting for next idle reviewer agent')).toBeInTheDocument()
  })
})
