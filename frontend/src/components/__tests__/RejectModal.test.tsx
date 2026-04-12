import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import RejectModal from '../RejectModal'

describe('RejectModal', () => {
  it('requires a reason before submit', () => {
    const onSubmit = vi.fn()

    render(
      <RejectModal
        open
        payload={{
          actor_type: 'employee',
          actor_id: 1,
          actor_label: 'Reviewer',
          from_status: 'Review',
          to_status: 'Coding',
          action: 'reject',
        }}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Return task' }))

    expect(screen.getByText('Reason is required.')).toBeInTheDocument()
    expect(onSubmit).not.toHaveBeenCalled()
  })

  it('submits note and evidence text', () => {
    const onSubmit = vi.fn()

    render(
      <RejectModal
        open
        payload={{
          actor_type: 'employee',
          actor_id: 1,
          actor_label: 'Reviewer',
          from_status: 'Review',
          to_status: 'Coding',
          action: 'reject',
        }}
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />,
    )

    fireEvent.change(screen.getByPlaceholderText('What still needs to change?'), {
      target: { value: 'Please address the failing case.' },
    })
    fireEvent.change(screen.getByPlaceholderText('Optional proof, logs, or review notes.'), {
      target: { value: 'console output' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Return task' }))

    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        note: 'Please address the failing case.',
        evidence_text: 'console output',
      }),
    )
  })
})
