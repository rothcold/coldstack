import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import EmployeeDetail from '../EmployeeDetail'

vi.mock('../LiveTerminal', () => ({
  default: ({ executionId }: { executionId: number }) => <div>terminal {executionId}</div>,
}))

const baseEmployee = {
  id: 1,
  name: 'Ava Agent',
  role: 'Coder',
  workflow_role: 'coder' as const,
  department: 'Engineering',
  agent_backend: 'claude_code',
  backend_available: true,
  custom_prompt: null,
  system_prompt: 'You are the coder.',
  status: 'idle' as const,
  created_at: '2026-04-16T00:00:00Z',
}

describe('EmployeeDetail stop and reset actions', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('renders stop for running executions and reset for recoverable states', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => [
            {
              id: 11,
              task_id: 42,
              employee_id: 1,
              started_at: '2026-04-16T00:00:00Z',
              finished_at: null,
              exit_code: null,
              status: 'running',
            },
          ],
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    const { unmount } = render(
      <EmployeeDetail
        employee={{ ...baseEmployee, status: 'working' }}
        onClose={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        onRefreshEmployees={async () => {}}
      />,
    )

    expect(await screen.findByRole('button', { name: 'Stop Execution' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Reset Agent' })).not.toBeInTheDocument()
    unmount()

    fetchMock.mockImplementation((input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => [],
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    const errorView = render(
      <EmployeeDetail
        employee={{ ...baseEmployee, status: 'error' }}
        onClose={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        onRefreshEmployees={async () => {}}
      />,
    )

    expect(await screen.findByRole('button', { name: 'Reset Agent' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Stop Execution' })).not.toBeInTheDocument()
    errorView.unmount()

    render(
      <EmployeeDetail
        employee={{ ...baseEmployee, status: 'working' }}
        onClose={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        onRefreshEmployees={async () => {}}
      />,
    )

    expect(await screen.findByRole('button', { name: 'Reset Agent' })).toBeInTheDocument()
  })

  it('stops a running execution and refreshes detail and parent state', async () => {
    const onRefreshEmployees = vi.fn(async () => {})
    let executions: Array<{
      id: number
      task_id: number
      employee_id: number
      started_at: string
      finished_at: string | null
      exit_code: number | null
      status: 'running' | 'cancelled'
    }> = [
      {
        id: 11,
        task_id: 42,
        employee_id: 1,
        started_at: '2026-04-16T00:00:00Z',
        finished_at: null,
        exit_code: null,
        status: 'running',
      },
    ]

    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => executions,
        })
      }
      if (url === '/api/executions/11/cancel' && init?.method === 'POST') {
        executions = [
          {
            id: 11,
            task_id: 42,
            employee_id: 1,
            started_at: '2026-04-16T00:00:00Z',
            finished_at: '2026-04-16T00:05:00Z',
            exit_code: null,
            status: 'cancelled',
          },
        ]
        return Promise.resolve({
          ok: true,
          text: async () => '',
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(
      <EmployeeDetail
        employee={{ ...baseEmployee, status: 'working' }}
        onClose={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        onRefreshEmployees={onRefreshEmployees}
      />,
    )

    fireEvent.click(await screen.findByRole('button', { name: 'Stop Execution' }))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/executions/11/cancel',
        expect.objectContaining({ method: 'POST' }),
      )
      expect(onRefreshEmployees).toHaveBeenCalledTimes(1)
    })
  })

  it('shows a visible error when reset conflicts', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => [],
        })
      }
      if (url === '/api/employees/1/reset' && init?.method === 'POST') {
        return Promise.resolve({
          ok: false,
          text: async () => 'Employee still has a running execution. Stop it first.',
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })
    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(
      <EmployeeDetail
        employee={{ ...baseEmployee, status: 'error' }}
        onClose={() => {}}
        onEdit={() => {}}
        onDelete={() => {}}
        onRefreshEmployees={async () => {}}
      />,
    )

    fireEvent.click(await screen.findByRole('button', { name: 'Reset Agent' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Employee still has a running execution. Stop it first.',
    )
  })
})
