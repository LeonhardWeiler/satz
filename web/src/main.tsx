import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import init, { Engine } from './engine/engine'
import { App } from './App'

await init()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App engine={new Engine()} />
  </StrictMode>,
)
