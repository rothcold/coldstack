import { useCallback, useEffect, useState } from 'react'
import type { Employee, WorkflowRole } from '../types'
import EmployeeCard from './EmployeeCard'
import EmployeeDetail from './EmployeeDetail'
import Modal from './Modal'

type EmployeeFormState = {
  name: string
  role: string
  workflow_role: WorkflowRole
  department: string
  agent_backend: string
  custom_prompt: string
}

const EMPTY_FORM: EmployeeFormState = {
  name: '',
  role: '',
  workflow_role: 'planner',
  department: '',
  agent_backend: 'claude_code',
  custom_prompt: '',
}

export default function CompanyView() {
  const [employees, setEmployees] = useState<Employee[]>([])
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const [modalOpen, setModalOpen] = useState(false)
  const [editingId, setEditingId] = useState<number | null>(null)
  const [form, setForm] = useState<EmployeeFormState>(EMPTY_FORM)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchEmployees = useCallback(async () => {
    try {
      const res = await fetch('/api/employees')
      const data = await res.json()
      setEmployees(data)
    } catch (err) {
      console.error('Failed to fetch employees', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchEmployees()
  }, [fetchEmployees])

  const selectedEmployee = employees.find(e => e.id === selectedId) ?? null

  const openCreate = () => {
    setEditingId(null)
    setForm(EMPTY_FORM)
    setError(null)
    setModalOpen(true)
  }

  const openEdit = (employee: Employee) => {
    setEditingId(employee.id)
    setForm({
      name: employee.name,
      role: employee.role,
      workflow_role: employee.workflow_role,
      department: employee.department,
      agent_backend: employee.agent_backend || 'claude_code',
      custom_prompt: employee.custom_prompt ?? '',
    })
    setError(null)
    setModalOpen(true)
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!form.name.trim() || !form.role.trim() || !form.department.trim()) {
      setError('Name, role, and department are required.')
      return
    }
    setSaving(true)
    setError(null)
    try {
      const payload = {
        name: form.name.trim(),
        role: form.role.trim(),
        workflow_role: form.workflow_role,
        department: form.department.trim(),
        agent_backend: form.agent_backend,
        custom_prompt: form.custom_prompt.trim() || null,
      }
      const url = editingId ? `/api/employees/${editingId}` : '/api/employees'
      const res = await fetch(url, {
        method: editingId ? 'PUT' : 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      })
      if (!res.ok) {
        const txt = await res.text()
        throw new Error(txt || `${res.status} ${res.statusText}`)
      }
      setModalOpen(false)
      await fetchEmployees()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save employee')
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (employee: Employee) => {
    if (!confirm(`Delete ${employee.name}? Running executions will be cancelled.`)) return
    try {
      const res = await fetch(`/api/employees/${employee.id}`, { method: 'DELETE' })
      if (!res.ok) throw new Error(`${res.status}`)
      if (selectedId === employee.id) setSelectedId(null)
      await fetchEmployees()
    } catch (err) {
      alert(`Failed to delete: ${err instanceof Error ? err.message : err}`)
    }
  }

  return (
    <div style={{ display: 'flex', gap: '1.5rem', height: 'calc(100vh - 8rem)' }}>
      {/* Left Panel - Employee Grid */}
      <div
        style={{
          flex: selectedEmployee ? '0 0 420px' : 1,
          transition: 'flex-basis 0.25s ease',
          display: 'flex',
          flexDirection: 'column',
          gap: '1rem',
          minWidth: 0,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h2 style={{ fontSize: '1.5rem', fontWeight: 700, letterSpacing: '-0.01em' }}>Agent roster & role setup</h2>
            <div style={{ fontSize: '0.85rem', color: 'var(--text-tertiary)', marginTop: '0.25rem' }}>
              {employees.length} {employees.length === 1 ? 'agent' : 'agents'}
            </div>
          </div>
          <button
            onClick={openCreate}
            style={{
              background: 'var(--accent-primary)',
              color: 'white',
              border: 'none',
              padding: '0.55rem 1rem',
              borderRadius: 'var(--radius-md)',
              fontWeight: 500,
              fontSize: '0.875rem',
              cursor: 'pointer',
            }}
          >
            + New Agent
          </button>
        </div>

        {loading ? (
          <div style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-tertiary)' }}>Loading…</div>
        ) : employees.length === 0 ? (
          <div
            style={{
              padding: '3rem 2rem',
              textAlign: 'center',
              color: 'var(--text-tertiary)',
              background: 'var(--bg-primary)',
              borderRadius: 'var(--radius-lg)',
              border: '1px dashed var(--border-color)',
            }}
          >
            No agents yet. Click <strong>+ New Agent</strong> to create one and assign a workflow role.
          </div>
        ) : (
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: selectedEmployee
                ? '1fr'
                : 'repeat(auto-fill, minmax(280px, 1fr))',
              gap: '0.9rem',
              overflowY: 'auto',
              paddingRight: '0.25rem',
              paddingBottom: '0.5rem',
            }}
          >
            {employees.map(employee => (
              <EmployeeCard
                key={employee.id}
                employee={employee}
                selected={selectedId === employee.id}
                onClick={() => setSelectedId(employee.id)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Right Panel - Employee Detail */}
      {selectedEmployee && (
        <div style={{ flex: 1, minWidth: 0 }}>
          <EmployeeDetail
            employee={selectedEmployee}
            onClose={() => setSelectedId(null)}
            onEdit={() => openEdit(selectedEmployee)}
            onDelete={() => handleDelete(selectedEmployee)}
            onRefreshEmployees={fetchEmployees}
          />
        </div>
      )}

      <Modal
        open={modalOpen}
        title={editingId ? 'Edit agent' : 'Create agent'}
        onClose={() => setModalOpen(false)}
        width={560}
      >
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
            <Field label="Name">
              <input
                type="text"
                value={form.name}
                onChange={e => setForm({ ...form, name: e.target.value })}
                placeholder="e.g. Alice"
                required
              />
            </Field>
            <Field label="Role">
              <input
                type="text"
                value={form.role}
                onChange={e => setForm({ ...form, role: e.target.value })}
                placeholder="e.g. Backend Engineer"
                required
              />
            </Field>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
            <Field label="Workflow role">
              <select
                value={form.workflow_role}
                onChange={e => setForm({ ...form, workflow_role: e.target.value as WorkflowRole })}
              >
                <option value="planner">planner</option>
                <option value="designer">designer</option>
                <option value="coder">coder</option>
                <option value="reviewer">reviewer</option>
                <option value="qa">qa</option>
                <option value="human">human</option>
              </select>
            </Field>
            <Field label="Department">
              <input
                type="text"
                value={form.department}
                onChange={e => setForm({ ...form, department: e.target.value })}
                placeholder="e.g. Engineering"
                required
              />
            </Field>
            <Field label="Agent Backend">
              <select
                value={form.agent_backend}
                onChange={e => setForm({ ...form, agent_backend: e.target.value })}
              >
                <option value="claude_code">claude_code</option>
              </select>
            </Field>
          </div>
          <Field label="Additional Instructions">
            <textarea
              value={form.custom_prompt}
              onChange={e => setForm({ ...form, custom_prompt: e.target.value })}
              placeholder="Add any task-specific instructions that should come after the role default prompt."
              rows={5}
              style={{ resize: 'vertical', minHeight: '6rem' }}
            />
          </Field>
          <div style={{ fontSize: '0.78rem', color: 'var(--text-tertiary)', marginTop: '-0.5rem' }}>
            The system will always prepend the default prompt for this workflow role. These instructions are appended after it.
          </div>

          {error && (
            <div
              style={{
                background: 'var(--status-offline-bg)',
                color: 'var(--status-offline)',
                padding: '0.6rem 0.8rem',
                borderRadius: 'var(--radius-md)',
                fontSize: '0.85rem',
              }}
            >
              {error}
            </div>
          )}

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '0.25rem' }}>
            <button
              type="button"
              onClick={() => setModalOpen(false)}
              style={{
                background: 'transparent',
                color: 'var(--text-secondary)',
                border: '1px solid var(--border-color)',
              }}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              style={{
                background: 'var(--accent-primary)',
                color: 'white',
                border: 'none',
                opacity: saving ? 0.7 : 1,
              }}
            >
              {saving ? 'Saving…' : editingId ? 'Save changes' : 'Create agent'}
            </button>
          </div>
        </form>
      </Modal>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem' }}>
      <span style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-secondary)', textTransform: 'uppercase', letterSpacing: '0.04em' }}>
        {label}
      </span>
      {children}
    </label>
  )
}
