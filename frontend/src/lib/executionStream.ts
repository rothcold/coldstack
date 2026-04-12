export interface ExecutionOutputEvent {
  execution_id: number
  chunk: string
  seq: number
  ts: string
}

export interface ExecutionStatusEvent {
  execution_id: number
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  exit_code?: number
}

interface ExecutionListener {
  onOutput?: (event: ExecutionOutputEvent) => void
  onStatus?: (event: ExecutionStatusEvent) => void
  onError?: () => void
}

interface ExecutionStream {
  source: EventSource
  listeners: Set<ExecutionListener>
  refs: number
  lastSeq: number
}

const streams = new Map<number, ExecutionStream>()

function closeStream(executionId: number) {
  const stream = streams.get(executionId)
  if (!stream) return
  stream.source.close()
  streams.delete(executionId)
}

function getOrCreateStream(executionId: number): ExecutionStream {
  const existing = streams.get(executionId)
  if (existing) return existing

  const source = new EventSource(`/api/executions/${executionId}/stream`)
  const stream: ExecutionStream = {
    source,
    listeners: new Set(),
    refs: 0,
    lastSeq: 0,
  }

  source.addEventListener('output', (event) => {
    try {
      const payload = JSON.parse((event as MessageEvent<string>).data) as ExecutionOutputEvent
      if (payload.seq <= stream.lastSeq) return
      stream.lastSeq = payload.seq
      stream.listeners.forEach((listener) => listener.onOutput?.(payload))
    } catch {
      stream.listeners.forEach((listener) => listener.onError?.())
    }
  })

  source.addEventListener('status', (event) => {
    try {
      const payload = JSON.parse((event as MessageEvent<string>).data) as ExecutionStatusEvent
      stream.listeners.forEach((listener) => listener.onStatus?.(payload))
      if (payload.status === 'completed' || payload.status === 'failed' || payload.status === 'cancelled') {
        closeStream(executionId)
      }
    } catch {
      stream.listeners.forEach((listener) => listener.onError?.())
    }
  })

  source.onerror = () => {
    stream.listeners.forEach((listener) => listener.onError?.())
  }

  streams.set(executionId, stream)
  return stream
}

export function subscribeExecution(
  executionId: number,
  listener: ExecutionListener,
): () => void {
  const stream = getOrCreateStream(executionId)
  stream.refs += 1
  stream.listeners.add(listener)

  return () => {
    const current = streams.get(executionId)
    if (!current) return
    current.listeners.delete(listener)
    current.refs -= 1
    if (current.refs <= 0) {
      closeStream(executionId)
    }
  }
}
