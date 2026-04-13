import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
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
})
