import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import EmployeeCard from '../EmployeeCard'

describe('EmployeeCard backend availability', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('shows backend unavailable badge when adapter is missing', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() =>
        Promise.resolve({
          status: 404,
          ok: false,
          json: async () => ({}),
        }),
      ) as unknown as typeof fetch,
    )

    render(
      <EmployeeCard
        employee={{
          id: 1,
          name: 'Alice',
          role: 'Coder',
          workflow_role: 'coder',
          department: 'Engineering',
          agent_backend: 'claude_code',
          backend_available: false,
          custom_prompt: null,
          system_prompt: 'You are the coder.',
          status: 'idle',
          created_at: '2026-04-12T00:00:00Z',
        }}
        onClick={() => {}}
        selected={false}
      />,
    )

    expect(await screen.findByText('backend unavailable')).toBeInTheDocument()
  })
})
