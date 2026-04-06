import { useState, useEffect } from 'react'

type TaskStatus = 'Pending' | 'Doing' | 'Finished' | 'Reviewing' | 'Done'

interface Subtask {
  id: number
  task_id: number
  title: string
  completed: boolean
  status: TaskStatus
  assignee?: string
}

interface Task {
  id: number
  task_id: string
  title: string
  description: string
  completed: boolean
  status: TaskStatus
  assignee?: string
  created_at: string
  subtasks: Subtask[]
}

interface Agent {
  id: number
  name: string
  cli: string
  system_prompt: string
  work_dir: string
  model?: string
  max_concurrency: number
  created_at: string
}


function App() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [newTaskId, setNewTaskId] = useState('')
  const [newTitle, setNewTitle] = useState('')
  const [newDesc, setNewDesc] = useState('')
  const [newAssignee, setNewAssignee] = useState('')
  const [editingId, setEditingId] = useState<number | null>(null)
  const [editTaskId, setEditTaskId] = useState('')
  const [editTitle, setEditTitle] = useState('')
  const [editDesc, setEditDesc] = useState('')
  const [editStatus, setEditStatus] = useState<TaskStatus>('Pending')
  const [editAssignee, setEditAssignee] = useState('')
  const [newSubtaskTitle, setNewSubtaskTitle] = useState<{ [key: number]: string }>({})
  const [newSubtaskAssignee, setNewSubtaskAssignee] = useState<{ [key: number]: string }>({})
  const [editingSubtaskId, setEditingSubtaskId] = useState<number | null>(null)
  const [editSubtaskTitle, setEditSubtaskTitle] = useState('')
  const [editSubtaskStatus, setEditSubtaskStatus] = useState<TaskStatus>('Pending')
  const [editSubtaskAssignee, setEditSubtaskAssignee] = useState('')

  // Agent state
  const [agents, setAgents] = useState<Agent[]>([])
  const [showAgents, setShowAgents] = useState(false)
  const [newAgentName, setNewAgentName] = useState('')
  const [newAgentCli, setNewAgentCli] = useState<string>('claude')
  const [newAgentPrompt, setNewAgentPrompt] = useState('')
  const [newAgentWorkDir, setNewAgentWorkDir] = useState('')
  const [newAgentModel, setNewAgentModel] = useState('')
  const [editingAgentId, setEditingAgentId] = useState<number | null>(null)
  const [editAgentName, setEditAgentName] = useState('')
  const [editAgentCli, setEditAgentCli] = useState<string>('claude')
  const [editAgentPrompt, setEditAgentPrompt] = useState('')
  const [editAgentWorkDir, setEditAgentWorkDir] = useState('')
  const [editAgentModel, setEditAgentModel] = useState('')
  const [editAgentConcurrency, setEditAgentConcurrency] = useState(1)

  const API_URL = '/api'

  const fetchTasks = async () => {
    try {
      const res = await fetch(`${API_URL}/tasks`)
      const data = await res.json()
      setTasks(data)
    } catch (err) {
      console.error('Failed to fetch tasks:', err)
    }
  }

  const fetchAgents = async () => {
    try {
      const res = await fetch(`${API_URL}/agents`)
      const data = await res.json()
      setAgents(data)
    } catch (err) {
      console.error('Failed to fetch agents:', err)
    }
  }

  useEffect(() => {
    fetchTasks()
    fetchAgents()
  }, [])

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newTitle.trim() || !newTaskId.trim()) {
      alert('Title and Task ID are required')
      return
    }

    try {
      const res = await fetch(`${API_URL}/tasks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          task_id: newTaskId,
          title: newTitle, 
          description: newDesc,
          assignee: newAssignee || null
        }),
      })
      if (res.ok) {
        setNewTaskId('')
        setNewTitle('')
        setNewDesc('')
        setNewAssignee('')
        fetchTasks()
      } else if (res.status === 409) {
        alert('Task ID already exists')
      }
    } catch (err) {
      console.error('Failed to add task:', err)
    }
  }

  const handleToggle = async (task: Task) => {
    try {
      await fetch(`${API_URL}/tasks/${task.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ completed: !task.completed }),
      })
      fetchTasks()
    } catch (err) {
      console.error('Failed to update task:', err)
    }
  }

  const handleDelete = async (id: number) => {
    try {
      await fetch(`${API_URL}/tasks/${id}`, {
        method: 'DELETE',
      })
      fetchTasks()
    } catch (err) {
      console.error('Failed to delete task:', err)
    }
  }

  const startEdit = (task: Task) => {
    setEditingId(task.id)
    setEditTaskId(task.task_id as string)
    setEditTitle(task.title as string)
    setEditDesc(task.description as string)
    setEditStatus(task.status)
    setEditAssignee((task.assignee as string) || '')
  }

  const handleSaveEdit = async (id: number) => {
    try {
      const res = await fetch(`${API_URL}/tasks/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          task_id: editTaskId,
          title: editTitle, 
          description: editDesc, 
          status: editStatus,
          assignee: editAssignee || null
        }),
      })
      if (res.ok) {
        setEditingId(null)
        fetchTasks()
      } else if (res.status === 409) {
        alert('Task ID already exists')
      }
    } catch (err) {
      console.error('Failed to save edit:', err)
    }
  }

  const handleAddSubtask = async (taskId: number) => {
    const title = newSubtaskTitle[taskId]
    const assignee = newSubtaskAssignee[taskId]
    if (!title || !title.trim()) return

    try {
      const res = await fetch(`${API_URL}/tasks/${taskId}/subtasks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title, assignee: assignee || null }),
      })
      if (res.ok) {
        setNewSubtaskTitle(prev => ({ ...prev, [taskId]: '' }))
        setNewSubtaskAssignee(prev => ({ ...prev, [taskId]: '' }))
        fetchTasks()
      }
    } catch (err) {
      console.error('Failed to add subtask:', err)
    }
  }

  const handleToggleSubtask = async (taskId: number, subtaskId: number) => {
    try {
      await fetch(`${API_URL}/tasks/${taskId}/subtasks/${subtaskId}/toggle`, {
        method: 'POST',
      })
      fetchTasks()
    } catch (err) {
      console.error('Failed to toggle subtask:', err)
    }
  }

  const startSubtaskEdit = (sub: Subtask) => {
    setEditingSubtaskId(sub.id)
    setEditSubtaskTitle(sub.title)
    setEditSubtaskStatus(sub.status)
    setEditSubtaskAssignee(sub.assignee || '')
  }

  const handleSaveSubtaskEdit = async (taskId: number, subtaskId: number) => {
    try {
      await fetch(`${API_URL}/tasks/${taskId}/subtasks/${subtaskId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          title: editSubtaskTitle, 
          status: editSubtaskStatus,
          assignee: editSubtaskAssignee || null
        }),
      })
      setEditingSubtaskId(null)
      fetchTasks()
    } catch (err) {
      console.error('Failed to save subtask edit:', err)
    }
  }

  const handleCancelEdit = () => {
    setEditingId(null)
    setEditTitle('')
    setEditDesc('')
    setEditAssignee('')
  }

  // Agent handlers
  const handleAddAgent = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newAgentName.trim()) return
    try {
      const res = await fetch(`${API_URL}/agents`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newAgentName,
          cli: newAgentCli,
          system_prompt: newAgentPrompt || null,
          work_dir: newAgentWorkDir || null,
          model: newAgentModel || null,
        }),
      })
      if (res.ok) {
        setNewAgentName(''); setNewAgentCli('claude'); setNewAgentPrompt(''); setNewAgentWorkDir(''); setNewAgentModel('')
        fetchAgents()
      } else if (res.status === 409) {
        alert('Agent name already exists')
      }
    } catch (err) { console.error('Failed to add agent:', err) }
  }

  const handleDeleteAgent = async (id: number) => {
    try {
      await fetch(`${API_URL}/agents/${id}`, { method: 'DELETE' })
      fetchAgents()
    } catch (err) { console.error('Failed to delete agent:', err) }
  }

  const startAgentEdit = (agent: Agent) => {
    setEditingAgentId(agent.id)
    setEditAgentName(agent.name)
    setEditAgentCli(agent.cli)
    setEditAgentPrompt(agent.system_prompt)
    setEditAgentWorkDir(agent.work_dir)
    setEditAgentModel(agent.model || '')
    setEditAgentConcurrency(agent.max_concurrency)
  }

  const handleSaveAgentEdit = async (id: number) => {
    try {
      const res = await fetch(`${API_URL}/agents/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: editAgentName,
          cli: editAgentCli,
          system_prompt: editAgentPrompt,
          work_dir: editAgentWorkDir,
          model: editAgentModel || null,
          max_concurrency: editAgentConcurrency,
        }),
      })
      if (res.ok) { setEditingAgentId(null); fetchAgents() }
      else if (res.status === 409) { alert('Agent name already exists') }
    } catch (err) { console.error('Failed to save agent:', err) }
  }

  return (
    <div style={{ maxWidth: '600px', margin: '0 auto', padding: '2rem', fontFamily: 'system-ui, sans-serif' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1.5rem' }}>
        <h1 style={{ fontSize: '1.5rem', fontWeight: 'bold', margin: 0 }}>Task Manager</h1>
        <button
          onClick={() => setShowAgents(!showAgents)}
          style={{ padding: '0.4rem 0.8rem', background: showAgents ? '#6366f1' : '#8b5cf6', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.85rem' }}
        >
          {showAgents ? 'Hide Agents' : `Agents (${agents.length})`}
        </button>
      </div>

      {showAgents && (
        <div style={{ marginBottom: '1.5rem', padding: '1rem', background: '#f5f3ff', borderRadius: '8px', border: '1px solid #e0e7ff' }}>
          <h2 style={{ fontSize: '1.1rem', fontWeight: 'bold', marginBottom: '0.75rem', color: '#4338ca' }}>Agents</h2>
          <form onSubmit={handleAddAgent} style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', marginBottom: '0.75rem' }}>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <input type="text" placeholder="Agent name" value={newAgentName} onChange={(e) => setNewAgentName(e.target.value)}
                style={{ flex: 1, padding: '0.4rem', fontSize: '0.85rem', border: '1px solid #c7d2fe', borderRadius: '4px' }} />
              <select value={newAgentCli} onChange={(e) => setNewAgentCli(e.target.value)}
                style={{ padding: '0.4rem', fontSize: '0.85rem', border: '1px solid #c7d2fe', borderRadius: '4px' }}>
                <option value="claude">Claude</option>
                <option value="gemini">Gemini</option>
              </select>
              <input type="text" placeholder="Model (optional)" value={newAgentModel} onChange={(e) => setNewAgentModel(e.target.value)}
                style={{ width: '120px', padding: '0.4rem', fontSize: '0.85rem', border: '1px solid #c7d2fe', borderRadius: '4px' }} />
            </div>
            <input type="text" placeholder="Working directory (optional)" value={newAgentWorkDir} onChange={(e) => setNewAgentWorkDir(e.target.value)}
              style={{ padding: '0.4rem', fontSize: '0.85rem', border: '1px solid #c7d2fe', borderRadius: '4px' }} />
            <textarea placeholder="System prompt / role description" value={newAgentPrompt} onChange={(e) => setNewAgentPrompt(e.target.value)}
              style={{ padding: '0.4rem', fontSize: '0.85rem', border: '1px solid #c7d2fe', borderRadius: '4px', minHeight: '3rem' }} />
            <button type="submit" style={{ padding: '0.4rem 0.8rem', background: '#6366f1', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', alignSelf: 'flex-start' }}>
              Add Agent
            </button>
          </form>

          <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
            {agents.map(agent => (
              <li key={agent.id} style={{ padding: '0.6rem', borderBottom: '1px solid #e0e7ff', background: editingAgentId === agent.id ? '#ede9fe' : 'transparent' }}>
                {editingAgentId === agent.id ? (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
                    <div style={{ display: 'flex', gap: '0.4rem' }}>
                      <input type="text" value={editAgentName} onChange={(e) => setEditAgentName(e.target.value)}
                        style={{ flex: 1, padding: '0.3rem', fontSize: '0.85rem' }} />
                      <select value={editAgentCli} onChange={(e) => setEditAgentCli(e.target.value)}
                        style={{ padding: '0.3rem', fontSize: '0.85rem' }}>
                        <option value="claude">Claude</option>
                        <option value="gemini">Gemini</option>
                      </select>
                      <input type="text" placeholder="Model" value={editAgentModel} onChange={(e) => setEditAgentModel(e.target.value)}
                        style={{ width: '100px', padding: '0.3rem', fontSize: '0.85rem' }} />
                      <input type="number" value={editAgentConcurrency} onChange={(e) => setEditAgentConcurrency(Number(e.target.value))} min={1}
                        style={{ width: '50px', padding: '0.3rem', fontSize: '0.85rem' }} title="Max concurrency" />
                    </div>
                    <input type="text" value={editAgentWorkDir} onChange={(e) => setEditAgentWorkDir(e.target.value)} placeholder="Working directory"
                      style={{ padding: '0.3rem', fontSize: '0.85rem' }} />
                    <textarea value={editAgentPrompt} onChange={(e) => setEditAgentPrompt(e.target.value)} placeholder="System prompt"
                      style={{ padding: '0.3rem', fontSize: '0.85rem', minHeight: '3rem' }} />
                    <div style={{ display: 'flex', gap: '0.4rem' }}>
                      <button onClick={() => handleSaveAgentEdit(agent.id)} style={{ padding: '0.25rem 0.5rem', background: '#22c55e', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.8rem' }}>Save</button>
                      <button onClick={() => setEditingAgentId(null)} style={{ padding: '0.25rem 0.5rem', background: '#666', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.8rem' }}>Cancel</button>
                    </div>
                  </div>
                ) : (
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ fontWeight: 500 }}>{agent.name}</span>
                    <span style={{ fontSize: '0.7rem', padding: '0.1rem 0.4rem', background: agent.cli === 'claude' ? '#dbeafe' : '#dcfce7', borderRadius: '999px', color: agent.cli === 'claude' ? '#1e40af' : '#166534' }}>{agent.cli}</span>
                    {agent.model && <span style={{ fontSize: '0.7rem', color: '#6b7280' }}>{agent.model}</span>}
                    {agent.system_prompt && <span style={{ fontSize: '0.7rem', color: '#9ca3af', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '150px' }} title={agent.system_prompt}>{agent.system_prompt}</span>}
                    <div style={{ marginLeft: 'auto', display: 'flex', gap: '0.3rem' }}>
                      <button onClick={() => startAgentEdit(agent)} style={{ padding: '0.15rem 0.4rem', background: '#f59e0b', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.7rem' }}>Edit</button>
                      <button onClick={() => handleDeleteAgent(agent.id)} style={{ padding: '0.15rem 0.4rem', background: '#ef4444', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.7rem' }}>Delete</button>
                    </div>
                  </div>
                )}
              </li>
            ))}
          </ul>
          {agents.length === 0 && <p style={{ color: '#9ca3af', fontSize: '0.85rem', margin: '0.5rem 0 0' }}>No agents yet.</p>}
        </div>
      )}

      <form onSubmit={handleAdd} style={{ marginBottom: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input
            type="text"
            placeholder="Task ID (e.g. TASK-1)"
            value={newTaskId}
            onChange={(e) => setNewTaskId(e.target.value)}
            style={{ width: '120px', padding: '0.5rem', fontSize: '1rem', border: '1px solid #ccc', borderRadius: '4px' }}
          />
          <input
            type="text"
            placeholder="Task title"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid #ccc', borderRadius: '4px' }}
          />
          <select
            value={newAssignee}
            onChange={(e) => setNewAssignee(e.target.value)}
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid #ccc', borderRadius: '4px' }}
          >
            <option value="">No assignee</option>
            {agents.map(a => <option key={a.id} value={a.name}>{a.name} ({a.cli})</option>)}
          </select>
        </div>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input
            type="text"
            placeholder="Description (optional)"
            value={newDesc}
            onChange={(e) => setNewDesc(e.target.value)}
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid #ccc', borderRadius: '4px' }}
          />
          <button
            type="submit"
            style={{ padding: '0.5rem 1rem', background: '#2563eb', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
          >
            Add
          </button>
        </div>
      </form>

      <ul style={{ listStyle: 'none', padding: 0 }}>
        {tasks.map((task) => (
          <li
            key={task.id}
            style={{
              padding: '1rem',
              borderBottom: '1px solid #eee',
              background: editingId === task.id ? '#f9f9f9' : 'white',
              display: 'flex',
              flexDirection: 'column',
              gap: '0.75rem',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: '0.75rem' }}>
              <input
                type="checkbox"
                checked={task.completed}
                onChange={() => handleToggle(task)}
                style={{ width: '1.2rem', height: '1.2rem', marginTop: '0.2rem' }}
              />

              {editingId === task.id ? (
                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                  <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <input
                      type="text"
                      value={editTaskId}
                      onChange={(e) => setEditTaskId(e.target.value)}
                      style={{ width: '100px', padding: '0.25rem', fontSize: '1rem' }}
                      placeholder="ID"
                    />
                    <input
                      type="text"
                      value={editTitle}
                      onChange={(e) => setEditTitle(e.target.value)}
                      style={{ flex: 1, padding: '0.25rem', fontSize: '1rem' }}
                      placeholder="Title"
                    />
                    <select
                      value={editAssignee}
                      onChange={(e) => setEditAssignee(e.target.value)}
                      style={{ flex: 1, padding: '0.25rem', fontSize: '1rem' }}
                    >
                      <option value="">No assignee</option>
                      {agents.map(a => <option key={a.id} value={a.name}>{a.name} ({a.cli})</option>)}
                    </select>
                  </div>
                  <textarea
                    value={editDesc}
                    onChange={(e) => setEditDesc(e.target.value)}
                    style={{ padding: '0.25rem', fontSize: '0.875rem', color: '#666', minHeight: '3rem' }}
                    placeholder="Description"
                  />
                  <select
                    value={editStatus}
                    onChange={(e) => setEditStatus(e.target.value as TaskStatus)}
                    style={{ padding: '0.25rem', fontSize: '0.875rem' }}
                  >
                    <option value="Pending">Pending</option>
                    <option value="Doing">Doing</option>
                    <option value="Finished">Finished</option>
                    <option value="Reviewing">Reviewing</option>
                    <option value="Done">Done</option>
                  </select>
                  <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <button
                      onClick={() => handleSaveEdit(task.id)}
                      style={{ padding: '0.25rem 0.5rem', background: '#22c55e', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
                    >
                      Save
                    </button>
                    <button
                      onClick={handleCancelEdit}
                      style={{ padding: '0.25rem 0.5rem', background: '#666', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <>
                  <div style={{ flex: 1 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                      <span style={{ fontSize: '0.75rem', fontWeight: 'bold', color: '#666' }}>{task.task_id as string}</span>
                      <div style={{ fontWeight: task.completed ? 'normal' : '500', textDecoration: task.completed ? 'line-through' : 'none', color: task.completed ? '#999' : '#333' }}>
                        {task.title as string}
                      </div>
                      <span style={{ fontSize: '0.7rem', padding: '0.1rem 0.4rem', background: '#e5e7eb', borderRadius: '999px', color: '#4b5563' }}>
                        {task.status}
                      </span>
                      {task.assignee && (
                        <span style={{ fontSize: '0.7rem', color: '#2563eb' }}>
                          @{task.assignee}
                        </span>
                      )}
                    </div>
                    {task.description && (
                      <div style={{ fontSize: '0.875rem', color: '#666', marginTop: '0.25rem' }}>
                        {task.description}
                      </div>
                    )}
                    <div style={{ fontSize: '0.75rem', color: '#999', marginTop: '0.25rem' }}>
                      {new Date(task.created_at).toLocaleString()}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <button
                      onClick={() => startEdit(task)}
                      style={{ padding: '0.25rem 0.5rem', background: '#f59e0b', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.75rem' }}
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(task.id)}
                      style={{ padding: '0.25rem 0.5rem', background: '#ef4444', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.75rem' }}
                    >
                      Delete
                    </button>
                  </div>
                </>
              )}
            </div>

            {/* Subtasks Section */}
            {!editingId && (
              <div style={{ marginLeft: '2rem', padding: '0.5rem', borderLeft: '2px solid #eee' }}>
                <div style={{ fontSize: '0.8rem', fontWeight: 'bold', marginBottom: '0.5rem', color: '#666' }}>Subtasks</div>
                <ul style={{ listStyle: 'none', padding: 0, marginBottom: '0.5rem' }}>
                  {task.subtasks.map(sub => (
                    <li key={sub.id} style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem', marginBottom: '0.5rem' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.875rem' }}>
                        <input
                          type="checkbox"
                          checked={sub.completed}
                          onChange={() => handleToggleSubtask(task.id, sub.id)}
                        />
                        {editingSubtaskId === sub.id ? (
                          <div style={{ flex: 1, display: 'flex', gap: '0.25rem', alignItems: 'center' }}>
                            <input
                              type="text"
                              value={editSubtaskTitle}
                              onChange={(e) => setEditSubtaskTitle(e.target.value)}
                              style={{ flex: 1, padding: '0.1rem', fontSize: '0.8rem' }}
                            />
                            <input
                              type="text"
                              value={editSubtaskAssignee}
                              onChange={(e) => setEditSubtaskAssignee(e.target.value)}
                              style={{ width: '80px', padding: '0.1rem', fontSize: '0.8rem' }}
                              placeholder="Assignee"
                            />
                            <select
                              value={editSubtaskStatus}
                              onChange={(e) => setEditSubtaskStatus(e.target.value as TaskStatus)}
                              style={{ padding: '0.1rem', fontSize: '0.7rem' }}
                            >
                              <option value="Pending">Pending</option>
                              <option value="Doing">Doing</option>
                              <option value="Finished">Finished</option>
                              <option value="Reviewing">Reviewing</option>
                              <option value="Done">Done</option>
                            </select>
                            <button onClick={() => handleSaveSubtaskEdit(task.id, sub.id)} style={{ fontSize: '0.7rem', padding: '0.1rem 0.3rem', background: '#22c55e', color: 'white', border: 'none', borderRadius: '2px' }}>Save</button>
                            <button onClick={() => setEditingSubtaskId(null)} style={{ fontSize: '0.7rem', padding: '0.1rem 0.3rem', background: '#666', color: 'white', border: 'none', borderRadius: '2px' }}>X</button>
                          </div>
                        ) : (
                          <>
                            <span style={{ textDecoration: sub.completed ? 'line-through' : 'none', color: sub.completed ? '#999' : '#333' }}>
                              {sub.title}
                            </span>
                            <span style={{ fontSize: '0.65rem', padding: '0.05rem 0.3rem', background: '#f3f4f6', borderRadius: '999px', color: '#6b7280' }}>
                              {sub.status}
                            </span>
                            {sub.assignee && (
                              <span style={{ fontSize: '0.65rem', color: '#3b82f6' }}>
                                @{sub.assignee}
                              </span>
                            )}
                            <button
                              onClick={() => startSubtaskEdit(sub)}
                              style={{ fontSize: '0.65rem', background: 'none', border: 'none', color: '#999', cursor: 'pointer', marginLeft: 'auto' }}
                            >
                              Edit
                            </button>
                          </>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <input
                    type="text"
                    placeholder="New subtask title"
                    value={newSubtaskTitle[task.id] || ''}
                    onChange={(e) => setNewSubtaskTitle(prev => ({ ...prev, [task.id]: e.target.value }))}
                    style={{ flex: 2, padding: '0.2rem', fontSize: '0.8rem', border: '1px solid #ddd', borderRadius: '4px' }}
                  />
                  <input
                    type="text"
                    placeholder="Assignee"
                    value={newSubtaskAssignee[task.id] || ''}
                    onChange={(e) => setNewSubtaskAssignee(prev => ({ ...prev, [task.id]: e.target.value }))}
                    style={{ flex: 1, padding: '0.2rem', fontSize: '0.8rem', border: '1px solid #ddd', borderRadius: '4px' }}
                  />
                  <button
                    onClick={() => handleAddSubtask(task.id)}
                    style={{ padding: '0.2rem 0.5rem', background: '#10b981', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer', fontSize: '0.8rem' }}
                  >
                    Add
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>

      {tasks.length === 0 && (
        <p style={{ textAlign: 'center', color: '#999', marginTop: '2rem' }}>No tasks yet. Add one above!</p>
      )}
    </div>
  )
}

export default App
