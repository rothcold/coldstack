import { useEffect, useState, useRef } from 'react'
import { subscribeExecution } from '../lib/executionStream'

interface LiveTerminalProps {
  executionId: number
}

export default function LiveTerminal({ executionId }: LiveTerminalProps) {
  const [output, setOutput] = useState<string>('')
  const [streamStatus, setStreamStatus] = useState<string>('running')
  const terminalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setOutput('')
    setStreamStatus('running')

    return subscribeExecution(executionId, {
      onOutput: (event) => {
        setOutput((prev) => prev + event.chunk + '\n')
      },
      onStatus: (event) => {
        setStreamStatus(event.status)
      },
      onError: () => {
        setStreamStatus('failed')
      },
    })
  }, [executionId])

  // Auto-scroll
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [output])

  return (
    <div 
      ref={terminalRef}
      role="log"
      aria-live="polite"
      aria-atomic="false"
      aria-label="Agent execution terminal output"
      style={{
        background: 'var(--terminal-bg)',
        color: 'var(--terminal-text)',
        fontFamily: 'var(--font-mono)',
        fontSize: '0.85rem',
        padding: '1rem',
        borderRadius: 'var(--radius-md)',
        border: '1px solid var(--terminal-border)',
        height: '300px',
        overflowY: 'auto',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all'
      }}
    >
      {output || (streamStatus === 'running' ? 'Waiting for output...' : `Execution ${streamStatus}.`)}
    </div>
  )
}
