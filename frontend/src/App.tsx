import { BrowserRouter, Routes, Route, NavLink } from 'react-router-dom'
import TasksView from './components/TasksView'
import CompanyView from './components/CompanyView'
import './App.css'

function App() {
  return (
    <BrowserRouter>
      <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
        <header style={{ 
          background: 'var(--bg-primary)', 
          borderBottom: '1px solid var(--border-color)',
          padding: '1rem 2rem',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}>
          <div style={{ fontWeight: 'bold', fontSize: '1.25rem', color: 'var(--text-primary)' }}>
            Agent Task Manager
          </div>
          <nav style={{ display: 'flex', gap: '1rem' }}>
            <NavLink 
              to="/" 
              style={({ isActive }) => ({
                textDecoration: 'none',
                padding: '0.5rem 1rem',
                borderRadius: 'var(--radius-md)',
                color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
                background: isActive ? 'var(--accent-light)' : 'transparent',
                fontWeight: isActive ? '500' : 'normal'
              })}
            >
              Tasks
            </NavLink>
            <NavLink 
              to="/company" 
              style={({ isActive }) => ({
                textDecoration: 'none',
                padding: '0.5rem 1rem',
                borderRadius: 'var(--radius-md)',
                color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
                background: isActive ? 'var(--accent-light)' : 'transparent',
                fontWeight: isActive ? '500' : 'normal'
              })}
            >
              Company
            </NavLink>
          </nav>
        </header>

        <main style={{ flex: 1, padding: '2rem' }}>
          <Routes>
            <Route path="/" element={<TasksView />} />
            <Route path="/company" element={<CompanyView />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  )
}

export default App
