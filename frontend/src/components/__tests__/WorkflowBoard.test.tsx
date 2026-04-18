import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TasksView from '../TasksView'

describe('Workflow board priority', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      writable: true,
      value: 1280,
    })
    vi.restoreAllMocks()
  })

  it('surfaces returned work ahead of normal tasks', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/tasks') {
        return Promise.resolve({
          ok: true,
          json: async () => [
            {
              id: 2,
              task_id: 'T-NORMAL',
              title: 'Normal task',
              status: 'Coding',
              board_group: 'Build',
              assignee: 'Owner',
              archived: false,
              needs_attention: false,
              waiting_for_human: false,
              waiting_for_agent: false,
              rejection_count: 0,
              latest_event_summary: 'Moved to coding',
            },
            {
              id: 1,
              task_id: 'T-PRIORITY',
              title: 'Returned task',
              status: 'Coding',
              board_group: 'Build',
              assignee: 'Owner',
              archived: false,
              needs_attention: true,
              waiting_for_human: false,
              waiting_for_agent: false,
              rejection_count: 1,
              latest_event_summary: 'Returned by QA',
            },
            {
              id: 3,
              task_id: 'T-DONE',
              title: 'Done task',
              status: 'Done',
              board_group: 'Done',
              assignee: 'Owner',
              archived: false,
              needs_attention: false,
              waiting_for_human: false,
              waiting_for_agent: false,
              rejection_count: 1,
              latest_event_summary: 'Moved to Done',
            },
          ],
        })
      }
      if (url === '/api/employees') {
        return Promise.resolve({
          ok: true,
          json: async () => [],
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(<TasksView />)

    const returned = await screen.findByText('Returned task')
    const normal = await screen.findByText('Normal task')
    expect(await screen.findByText('Done task')).toBeInTheDocument()
    const relation = returned.compareDocumentPosition(normal)

    expect(relation & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })
})
