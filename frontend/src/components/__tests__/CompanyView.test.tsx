import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('../../lib/executionStream', () => ({
  subscribeExecution: () => () => {},
}))
import CompanyView from '../CompanyView'

describe('CompanyView additional instructions', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('uses additional instructions semantics for create and edit flows', async () => {
    const employees = [
      {
        id: 1,
        name: 'Rita Reviewer',
        role: 'Reviewer',
        workflow_role: 'reviewer',
        department: 'QA',
        agent_backend: 'claude_code',
        backend_available: true,
        custom_prompt: 'Focus on regressions only',
        system_prompt:
          'You are the reviewer. Review code changes only. Find bugs, regressions, missing edge cases, and test gaps. Do not rewrite the feature or implement unrelated changes.\n\nFocus on regressions only',
        status: 'idle',
        created_at: '2026-04-13T00:00:00Z',
      },
    ]

    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url === '/api/employees' && !init) {
        return Promise.resolve({
          ok: true,
          json: async () => employees,
        })
      }
      if (url === '/api/employees' && init?.method === 'POST') {
        return Promise.resolve({
          ok: true,
          json: async () => ({ id: 2 }),
        })
      }
      if (url === '/api/employees/1' && init?.method === 'PUT') {
        return Promise.resolve({
          ok: true,
          json: async () => ({ id: 1 }),
        })
      }
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => [],
        })
      }
      if (url === '/api/employees/1/current_execution') {
        return Promise.resolve({
          status: 404,
          ok: false,
          json: async () => ({}),
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(<CompanyView />)

    fireEvent.click(await screen.findByRole('button', { name: '+ New Agent' }))

    expect(screen.getByText('Additional Instructions')).toBeInTheDocument()
    expect(
      screen.getByText(/default prompt for this workflow role/i),
    ).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Name'), {
      target: { value: 'Cody Coder' },
    })
    fireEvent.change(screen.getByLabelText('Role'), {
      target: { value: 'Coder' },
    })
    fireEvent.change(screen.getByLabelText('Department'), {
      target: { value: 'Engineering' },
    })
    fireEvent.change(screen.getByLabelText('Additional Instructions'), {
      target: { value: 'Only touch backend/src' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create agent' }))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/employees',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            name: 'Cody Coder',
            role: 'Coder',
            workflow_role: 'planner',
            department: 'Engineering',
            agent_backend: 'claude_code',
            custom_prompt: 'Only touch backend/src',
          }),
        }),
      )
    })

    fireEvent.click(await screen.findByText('Rita Reviewer'))
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }))

    const textarea = screen.getByLabelText('Additional Instructions')
    expect(textarea).toHaveValue('Focus on regressions only')
  })

  it('refreshes the roster after stopping an execution from the detail panel', async () => {
    let employees = [
      {
        id: 1,
        name: 'Wendy Worker',
        role: 'Coder',
        workflow_role: 'coder',
        department: 'Engineering',
        agent_backend: 'claude_code',
        backend_available: true,
        custom_prompt: null,
        system_prompt: 'You are the coder.',
        status: 'working',
        created_at: '2026-04-16T00:00:00Z',
      },
    ]

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
        id: 41,
        task_id: 99,
        employee_id: 1,
        started_at: '2026-04-16T00:00:00Z',
        finished_at: null,
        exit_code: null,
        status: 'running',
      },
    ]

    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url === '/api/employees' && !init) {
        return Promise.resolve({
          ok: true,
          json: async () => employees,
        })
      }
      if (url === '/api/employees/1/executions') {
        return Promise.resolve({
          ok: true,
          json: async () => executions,
        })
      }
      if (url === '/api/employees/1/current_execution') {
        return Promise.resolve({
          status: executions.some(execution => execution.status === 'running') ? 200 : 404,
          ok: executions.some(execution => execution.status === 'running'),
          json: async () =>
            executions.some(execution => execution.status === 'running')
              ? {
                  execution_id: 41,
                  task_id: 99,
                  task_key: 'T-99',
                  task_title: 'Keep running',
                  started_at: '2026-04-16T00:00:00Z',
                }
              : {},
        })
      }
      if (url === '/api/executions/41/cancel' && init?.method === 'POST') {
        employees = [{ ...employees[0], status: 'idle' }]
        executions = [{ ...executions[0], status: 'cancelled', finished_at: '2026-04-16T00:01:00Z' }]
        return Promise.resolve({
          ok: true,
          text: async () => '',
        })
      }
      throw new Error(`Unexpected fetch: ${url}`)
    })

    vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch)

    render(<CompanyView />)

    fireEvent.click(await screen.findByText('Wendy Worker'))
    fireEvent.click(await screen.findByRole('button', { name: 'Stop Execution' }))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/executions/41/cancel',
        expect.objectContaining({ method: 'POST' }),
      )
    })

    await waitFor(() => {
      expect(fetchMock.mock.calls.filter(([url, init]) => String(url) === '/api/employees' && !init)).toHaveLength(2)
    })
  })
})
