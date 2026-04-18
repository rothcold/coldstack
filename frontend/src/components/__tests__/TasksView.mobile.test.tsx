import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TasksView from '../TasksView'

function setMobileViewport() {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: 480,
  })
}

describe('TasksView mobile workflow behavior', () => {
  beforeEach(() => {
    setMobileViewport()
    vi.restoreAllMocks()
  })

  it('uses lane switcher and opens detail sheet', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/tasks') {
        return Promise.resolve({
          ok: true,
          json: async () => [
            {
              id: 1,
              task_id: 'T-MOBILE',
              title: 'Mobile detail',
              status: 'NeedsHuman',
              board_group: 'Human',
              assignee: 'Owner',
              archived: false,
              needs_attention: false,
              waiting_for_human: true,
              waiting_for_agent: false,
              rejection_count: 0,
              latest_event_summary: 'Needs your decision',
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
      if (url === '/api/tasks/1') {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            task: {
              id: 1,
              task_id: 'T-MOBILE',
              title: 'Mobile detail',
              description: 'Open in sheet',
              source: '/tmp/project',
              archived: false,
              status: 'NeedsHuman',
              assignee: 'Owner',
              created_at: '2026-04-09T00:00:00Z',
              subtasks: [],
            },
            events: [],
            current_action_label: 'Needs your decision',
            current_action_hint: 'Only you can close the loop.',
            waiting_for_agent: false,
          }),
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(<TasksView />)

    expect(await screen.findByRole('button', { name: 'Plan' })).toBeInTheDocument()
    fireEvent.click(await screen.findByText('Mobile detail'))

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: 'Mobile detail' })).toBeInTheDocument()
    })
    expect(screen.getByText('Open in sheet')).toBeInTheDocument()
  })
})
