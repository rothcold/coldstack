import { useState } from 'react'
import type { TransitionPayload } from '../types'
import Modal from './Modal'

interface RejectModalProps {
  open: boolean
  payload: TransitionPayload | null
  onClose: () => void
  onSubmit: (payload: TransitionPayload) => void
}

export default function RejectModal({ open, payload, onClose, onSubmit }: RejectModalProps) {
  const [note, setNote] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [error, setError] = useState<string | null>(null)

  const handleClose = () => {
    setNote('')
    setEvidenceText('')
    setError(null)
    onClose()
  }

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault()
    if (!payload) return
    if (!note.trim()) {
      setError('Reason is required.')
      return
    }
    onSubmit({
      ...payload,
      note: note.trim(),
      evidence_text: evidenceText.trim() || null,
    })
    handleClose()
  }

  return (
    <Modal open={open} title="Return task to coding" onClose={handleClose} width={640}>
      <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <label style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem' }}>
          <span style={{ fontSize: '0.76rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>
            Reason
          </span>
          <textarea
            autoFocus
            value={note}
            onChange={(event) => setNote(event.target.value)}
            rows={4}
            placeholder="What still needs to change?"
            style={{ resize: 'vertical', minHeight: '7rem' }}
          />
        </label>

        <label style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem' }}>
          <span style={{ fontSize: '0.76rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--text-tertiary)' }}>
            Evidence text
          </span>
          <textarea
            value={evidenceText}
            onChange={(event) => setEvidenceText(event.target.value)}
            rows={5}
            placeholder="Optional proof, logs, or review notes."
            style={{ resize: 'vertical', minHeight: '8rem' }}
          />
        </label>

        {error && (
          <div
            style={{
              padding: '0.75rem 0.9rem',
              borderRadius: 'var(--radius-md)',
              background: 'rgba(239, 68, 68, 0.08)',
              color: 'rgb(185, 28, 28)',
              fontSize: '0.86rem',
            }}
          >
            {error}
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.6rem' }}>
          <button type="button" onClick={handleClose} style={{ background: 'transparent', border: '1px solid var(--border-color)' }}>
            Cancel
          </button>
          <button type="submit" style={{ background: 'rgb(185, 28, 28)', color: 'white', border: 'none', minHeight: 44 }}>
            Return task
          </button>
        </div>
      </form>
    </Modal>
  )
}
