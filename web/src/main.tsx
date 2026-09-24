import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import init, { version } from './engine/engine'
import { App } from './App'

await init()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App version={version()} />
  </StrictMode>,
)
