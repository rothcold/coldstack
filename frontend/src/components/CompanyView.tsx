import { useState, useEffect } from 'react'
import type { Employee } from '../types'
import EmployeeCard from './EmployeeCard'
import EmployeeDetail from './EmployeeDetail'

export default function CompanyView() {
  const [employees, setEmployees] = useState<Employee[]>([])
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/employees')
      .then(res => res.json())
      .then(data => {
        setEmployees(data)
        setLoading(false)
      })
      .catch(err => {
        console.error('Failed to fetch employees', err)
        setLoading(false)
      })
  }, [])

  const selectedEmployee = employees.find(e => e.id === selectedId)

  if (loading) {
    return <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-tertiary)' }}>Loading company directory...</div>
  }

  return (
    <div style={{ display: 'flex', gap: '2rem', height: 'calc(100vh - 120px)', maxWidth: '1200px', margin: '0 auto' }}>
      {/* Left Sidebar - Employee List */}
      <div style={{ 
        width: selectedId ? '350px' : '100%',
        transition: 'width 0.3s ease',
        display: 'flex',
        flexDirection: 'column',
        gap: '1rem',
        overflowY: 'auto',
        paddingRight: '0.5rem'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ margin: 0, fontSize: '1.25rem' }}>AI Employees</h2>
        </div>
        
        {employees.length === 0 ? (
          <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-tertiary)', background: 'var(--bg-primary)', borderRadius: 'var(--radius-lg)', border: '1px solid var(--border-color)' }}>
            No AI employees found in the company.
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
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
        <div style={{ flex: 1, minWidth: 0, transition: 'all 0.3s ease' }}>
          <EmployeeDetail 
            employee={selectedEmployee} 
            onClose={() => setSelectedId(null)} 
          />
        </div>
      )}
    </div>
  )
}
