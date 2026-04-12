import { BrowserRouter, Routes, Route, NavLink } from 'react-router-dom'
import { useEffect, useState } from 'react'
import TasksView from './components/TasksView'
import CompanyView from './components/CompanyView'
import './App.css'

type Theme = 'light' | 'dark'

function getInitialTheme(): Theme {
  const saved = window.localStorage.getItem('theme')
  if (saved === 'light' || saved === 'dark') {
    return saved
  }
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function App() {
  const [theme, setTheme] = useState<Theme>(getInitialTheme)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    window.localStorage.setItem('theme', theme)
  }, [theme])

  const navLinkStyle = ({ isActive }: { isActive: boolean }): React.CSSProperties => ({
    textDecoration: 'none',
    padding: '0.45rem 0.9rem',
    borderRadius: 'var(--radius-md)',
    color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
    background: isActive ? 'var(--accent-light)' : 'transparent',
    fontWeight: 500,
    fontSize: '0.875rem',
    transition: 'all 0.15s ease',
  })

  return (
    <BrowserRouter>
      <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column', background: 'var(--bg-secondary)' }}>
        <header
          style={{
            background: 'var(--bg-primary)',
            borderBottom: '1px solid var(--border-color)',
            padding: '0.75rem 1.5rem',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            position: 'sticky',
            top: 0,
            zIndex: 10,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '1.5rem' }}>
            <div style={{ fontWeight: 700, fontSize: '1.1rem', color: 'var(--text-primary)', letterSpacing: '-0.01em' }}>
              Coldstack
            </div>
            <nav style={{ display: 'flex', gap: '0.25rem' }}>
              <NavLink to="/" end style={navLinkStyle}>Workflow</NavLink>
              <NavLink to="/company" style={navLinkStyle}>Agent Roster</NavLink>
            </nav>
          </div>
          <button
            type="button"
            onClick={() => setTheme((current) => (current === 'light' ? 'dark' : 'light'))}
            style={{
              border: '1px solid var(--border-color)',
              background: 'var(--bg-tertiary)',
              color: 'var(--text-primary)',
              minHeight: 36,
              padding: '0.4rem 0.75rem',
            }}
            aria-label={`Switch to ${theme === 'light' ? 'dark' : 'light'} mode`}
          >
            {theme === 'light' ? 'Dark' : 'Light'}
          </button>
        </header>

        <main style={{ flex: 1, padding: '1.5rem 2rem', minHeight: 0 }}>
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
