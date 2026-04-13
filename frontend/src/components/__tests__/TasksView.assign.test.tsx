import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TasksView from '../TasksView'

describe('TasksView assignment filtering', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      writable: true,
      value: 1280,
    })
    vi.restoreAllMocks()
  })

  it('shows only agents matching the task workflow role in assign modal', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/tasks') {
        return Promise.resolve({
          ok: true,
          json: async () => [
            {
              id: 1,
              task_id: 'T-REVIEW',
              title: 'Review this task',
              status: 'Review',
              board_group: 'Review',
              assignee: null,
              archived: false,
              needs_attention: false,
              waiting_for_human: false,
              rejection_count: 0,
              latest_event_summary: 'Ready for review',
            },
          ],
        })
      }
      if (url === '/api/employees') {
        return Promise.resolve({
          ok: true,
          json: async () => [
            {
              id: 1,
              name: 'Rita Reviewer',
              role: 'Reviewer',
              workflow_role: 'reviewer',
              department: 'QA',
              agent_backend: 'claude_code',
              backend_available: true,
              status: 'idle',
              created_at: '2026-04-13T00:00:00Z',
            },
            {
              id: 2,
              name: 'Casey Coder',
              role: 'Coder',
              workflow_role: 'coder',
              department: 'Engineering',
              agent_backend: 'claude_code',
              backend_available: true,
              status: 'idle',
              created_at: '2026-04-13T00:00:00Z',
            },
          ],
        })
      }
      if (url === '/api/tasks/1') {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            task: {
              id: 1,
              task_id: 'T-REVIEW',
              title: 'Review this task',
              description: 'Needs a reviewer assignment.',
              archived: false,
              status: 'Review',
              assignee: null,
              created_at: '2026-04-13T00:00:00Z',
              subtasks: [],
            },
            events: [],
            current_action_label: 'Needs review',
            current_action_hint: null,
          }),
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(<TasksView />)

    fireEvent.click(await screen.findByText('Review this task'))
    fireEvent.click(await screen.findByRole('button', { name: 'Assign to Agent' }))

    const dialog = await screen.findByRole('dialog', { name: 'Assign T-REVIEW' })

    await waitFor(() => {
      expect(within(dialog).getByText(/reviewer · claude_code/i)).toBeInTheDocument()
    })
    expect(within(dialog).getByText(/Rita Reviewer/)).toBeInTheDocument()
    expect(within(dialog).queryByText(/Casey Coder/)).not.toBeInTheDocument()
  })
})
