import { useState, useEffect } from 'react'
import type { Task, Subtask, TaskStatus, Agent } from '../types'

export default function TasksView() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [agents, setAgents] = useState<Agent[]>([])
  
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

  return (
    <div style={{ maxWidth: '800px', margin: '0 auto', padding: '2rem' }}>
      <h1 style={{ fontSize: '1.5rem', fontWeight: 'bold', margin: '0 0 1.5rem' }}>Task Manager</h1>
      
      <form onSubmit={handleAdd} style={{ marginBottom: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem', background: 'var(--bg-primary)', padding: '1rem', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-color)' }}>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input
            type="text"
            placeholder="Task ID (e.g. TASK-1)"
            value={newTaskId}
            onChange={(e) => setNewTaskId(e.target.value)}
            style={{ width: '120px', padding: '0.5rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
          />
          <input
            type="text"
            placeholder="Task title"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
          />
          <select
            value={newAssignee}
            onChange={(e) => setNewAssignee(e.target.value)}
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
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
            style={{ flex: 1, padding: '0.5rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
          />
          <button
            type="submit"
            style={{ padding: '0.5rem 1rem', background: 'var(--accent-primary)', color: 'white', border: 'none', borderRadius: 'var(--radius-md)', cursor: 'pointer' }}
          >
            Add
          </button>
        </div>
      </form>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0, background: 'var(--bg-primary)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-color)', overflow: 'hidden' }}>
        {tasks.map((task, idx) => (
          <li
            key={task.id}
            style={{
              padding: '1rem',
              borderBottom: idx === tasks.length - 1 ? 'none' : '1px solid var(--border-color)',
              background: editingId === task.id ? 'var(--bg-tertiary)' : 'transparent',
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
                      style={{ width: '100px', padding: '0.4rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
                      placeholder="ID"
                    />
                    <input
                      type="text"
                      value={editTitle}
                      onChange={(e) => setEditTitle(e.target.value)}
                      style={{ flex: 1, padding: '0.4rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
                      placeholder="Title"
                    />
                    <select
                      value={editAssignee}
                      onChange={(e) => setEditAssignee(e.target.value)}
                      style={{ flex: 1, padding: '0.4rem', fontSize: '1rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
                    >
                      <option value="">No assignee</option>
                      {agents.map(a => <option key={a.id} value={a.name}>{a.name} ({a.cli})</option>)}
                    </select>
                  </div>
                  <textarea
                    value={editDesc}
                    onChange={(e) => setEditDesc(e.target.value)}
                    style={{ padding: '0.4rem', fontSize: '0.875rem', color: 'var(--text-secondary)', minHeight: '3rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
                    placeholder="Description"
                  />
                  <select
                    value={editStatus}
                    onChange={(e) => setEditStatus(e.target.value as TaskStatus)}
                    style={{ padding: '0.4rem', fontSize: '0.875rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-md)' }}
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
                      style={{ padding: '0.4rem 0.8rem', background: 'var(--status-working)', color: 'white', border: 'none', borderRadius: 'var(--radius-md)', cursor: 'pointer' }}
                    >
                      Save
                    </button>
                    <button
                      onClick={handleCancelEdit}
                      style={{ padding: '0.4rem 0.8rem', background: 'var(--text-tertiary)', color: 'white', border: 'none', borderRadius: 'var(--radius-md)', cursor: 'pointer' }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <>
                  <div style={{ flex: 1 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                      <span style={{ fontSize: '0.75rem', fontWeight: 'bold', color: 'var(--text-secondary)' }}>{task.task_id as string}</span>
                      <div style={{ fontWeight: task.completed ? 'normal' : '500', textDecoration: task.completed ? 'line-through' : 'none', color: task.completed ? 'var(--text-tertiary)' : 'var(--text-primary)' }}>
                        {task.title as string}
                      </div>
                      <span style={{ fontSize: '0.7rem', padding: '0.1rem 0.4rem', background: 'var(--bg-tertiary)', borderRadius: 'var(--radius-full)', color: 'var(--text-secondary)' }}>
                        {task.status}
                      </span>
                      {task.assignee && (
                        <span style={{ fontSize: '0.7rem', color: 'var(--accent-primary)' }}>
                          @{task.assignee}
                        </span>
                      )}
                    </div>
                    {task.description && (
                      <div style={{ fontSize: '0.875rem', color: 'var(--text-secondary)', marginTop: '0.25rem' }}>
                        {task.description}
                      </div>
                    )}
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-tertiary)', marginTop: '0.25rem' }}>
                      {new Date(task.created_at).toLocaleString()}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <button
                      onClick={() => startEdit(task)}
                      style={{ padding: '0.25rem 0.5rem', background: 'var(--status-reviewing)', color: 'white', border: 'none', borderRadius: 'var(--radius-sm)', cursor: 'pointer', fontSize: '0.75rem' }}
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(task.id)}
                      style={{ padding: '0.25rem 0.5rem', background: 'var(--status-offline)', color: 'white', border: 'none', borderRadius: 'var(--radius-sm)', cursor: 'pointer', fontSize: '0.75rem' }}
                    >
                      Delete
                    </button>
                  </div>
                </>
              )}
            </div>

            {/* Subtasks Section */}
            {!editingId && (
              <div style={{ marginLeft: '2rem', padding: '0.5rem', borderLeft: '2px solid var(--border-color)' }}>
                <div style={{ fontSize: '0.8rem', fontWeight: 'bold', marginBottom: '0.5rem', color: 'var(--text-secondary)' }}>Subtasks</div>
                <ul style={{ listStyle: 'none', padding: 0, margin: '0 0 0.5rem 0' }}>
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
                              style={{ flex: 1, padding: '0.1rem', fontSize: '0.8rem', border: '1px solid var(--border-color)' }}
                            />
                            <input
                              type="text"
                              value={editSubtaskAssignee}
                              onChange={(e) => setEditSubtaskAssignee(e.target.value)}
                              style={{ width: '80px', padding: '0.1rem', fontSize: '0.8rem', border: '1px solid var(--border-color)' }}
                              placeholder="Assignee"
                            />
                            <select
                              value={editSubtaskStatus}
                              onChange={(e) => setEditSubtaskStatus(e.target.value as TaskStatus)}
                              style={{ padding: '0.1rem', fontSize: '0.7rem', border: '1px solid var(--border-color)' }}
                            >
                              <option value="Pending">Pending</option>
                              <option value="Doing">Doing</option>
                              <option value="Finished">Finished</option>
                              <option value="Reviewing">Reviewing</option>
                              <option value="Done">Done</option>
                            </select>
                            <button onClick={() => handleSaveSubtaskEdit(task.id, sub.id)} style={{ fontSize: '0.7rem', padding: '0.1rem 0.3rem', background: 'var(--status-working)', color: 'white', border: 'none', borderRadius: 'var(--radius-sm)' }}>Save</button>
                            <button onClick={() => setEditingSubtaskId(null)} style={{ fontSize: '0.7rem', padding: '0.1rem 0.3rem', background: 'var(--text-tertiary)', color: 'white', border: 'none', borderRadius: 'var(--radius-sm)' }}>X</button>
                          </div>
                        ) : (
                          <>
                            <span style={{ textDecoration: sub.completed ? 'line-through' : 'none', color: sub.completed ? 'var(--text-tertiary)' : 'var(--text-primary)' }}>
                              {sub.title}
                            </span>
                            <span style={{ fontSize: '0.65rem', padding: '0.05rem 0.3rem', background: 'var(--bg-tertiary)', borderRadius: 'var(--radius-full)', color: 'var(--text-secondary)' }}>
                              {sub.status}
                            </span>
                            {sub.assignee && (
                              <span style={{ fontSize: '0.65rem', color: 'var(--accent-primary)' }}>
                                @{sub.assignee}
                              </span>
                            )}
                            <button
                              onClick={() => startSubtaskEdit(sub)}
                              style={{ fontSize: '0.65rem', background: 'none', border: 'none', color: 'var(--text-tertiary)', cursor: 'pointer', marginLeft: 'auto' }}
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
                    style={{ flex: 2, padding: '0.4rem', fontSize: '0.8rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-sm)' }}
                  />
                  <input
                    type="text"
                    placeholder="Assignee"
                    value={newSubtaskAssignee[task.id] || ''}
                    onChange={(e) => setNewSubtaskAssignee(prev => ({ ...prev, [task.id]: e.target.value }))}
                    style={{ flex: 1, padding: '0.4rem', fontSize: '0.8rem', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-sm)' }}
                  />
                  <button
                    onClick={() => handleAddSubtask(task.id)}
                    style={{ padding: '0.4rem 0.5rem', background: 'var(--status-working)', color: 'white', border: 'none', borderRadius: 'var(--radius-sm)', cursor: 'pointer', fontSize: '0.8rem' }}
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
        <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', marginTop: '2rem' }}>No tasks yet. Add one above!</p>
      )}
    </div>
  )
}
