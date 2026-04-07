import { useEffect, useState, useRef } from 'react'

interface LiveTerminalProps {
  executionId: number
}

export default function LiveTerminal({ executionId }: LiveTerminalProps) {
  const [output, setOutput] = useState<string>('')
  const terminalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setOutput('') // Reset output when execution changes
    
    // Load historical output first
    fetch(`/api/executions/${executionId}/output`)
      .then(res => res.json())
      .then(chunks => {
        const text = chunks.map((c: any) => c.chunk).join('')
        setOutput(text)
      })
      .catch(err => console.error('Failed to load execution output', err))

    // Stream new output via SSE
    const eventSource = new EventSource(`/api/executions/${executionId}/stream`)
    
    eventSource.onmessage = (e) => {
      setOutput(prev => prev + e.data)
    }
    
    eventSource.onerror = () => {
      // It's normal for SSE to close when execution is complete
      eventSource.close()
    }

    return () => {
      eventSource.close()
    }
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
      {output || 'Waiting for output...'}
    </div>
  )
}
