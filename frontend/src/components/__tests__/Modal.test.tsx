import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import Modal from '../Modal'

function ModalHarness() {
  const [value, setValue] = useState('')

  return (
    <Modal open title="Focus test" onClose={() => {}}>
      <input
        autoFocus
        aria-label="Task ID"
        value={value}
        onChange={(event) => setValue(event.target.value)}
      />
    </Modal>
  )
}

describe('Modal', () => {
  it('keeps focus on the autofocus input while typing across rerenders', async () => {
    render(<ModalHarness />)

    const input = await screen.findByRole('textbox', { name: 'Task ID' })
    expect(input).toHaveFocus()

    fireEvent.change(input, { target: { value: 'T-123' } })

    expect(input).toHaveFocus()
    expect(input).toHaveValue('T-123')
    expect(screen.getByRole('button', { name: 'Close' })).not.toHaveFocus()
  })

  it('handles escape through the latest onClose callback', async () => {
    function EscapeHarness() {
      const [calls, setCalls] = useState(0)
      const handleClose = vi.fn(() => setCalls((current) => current + 1))

      return (
        <>
          <span>{calls}</span>
          <Modal open title="Escape test" onClose={handleClose}>
            <input autoFocus aria-label="Field" />
          </Modal>
        </>
      )
    }

    render(<EscapeHarness />)

    fireEvent.keyDown(window, { key: 'Escape' })

    expect(await screen.findByText('1')).toBeInTheDocument()
  })
})
