import type { Agent, AgentConnection } from './model.ts'

const holding = (agent: Agent): boolean => agent.state === 'working' && !agent.paused

/** Agent holds keep the computer awake only while one works unpaused and the setting is on. */
export function agentsHold(agents: Agent[], enabled: boolean): boolean {
  return enabled && agents.some(holding)
}

/** "Claude Code", or for `other` its label only once. */
export function agentPlace(agent: Agent): string | null {
  return agent.label && agent.label !== agent.name ? agent.label : null
}

/** What Rust says holds the computer, for the demo bridge: "Claude Code works", "2 agents work". */
export function agentHolder(agents: Agent[]): string | null {
  const working = agents.filter(holding)
  const first = working[0]
  if (!first) return null
  return working.length > 1 ? `${working.length} agents work` : `${first.name} works`
}

/**
 * The status line leaves the project to the agents card below it; a screen reader hears it:
 * "Claude Code works in wick". Anything else stays as Rust phrased it.
 */
export function agentSpoken(heldBy: string, agents: Agent[]): string {
  const working = agents.filter(holding)
  const first = working[0]
  const place = first && working.length === 1 ? agentPlace(first) : null
  return first && place && heldBy === `${first.name} works` ? `${heldBy} in ${place}` : heldBy
}

/** What a pause by Stop waits for. */
export function agentPause(agents: Agent[]): string {
  const working = agents.filter(agent => agent.state === 'working')
  return working.length === 1 && working[0] ? `${working[0].name} finishes` : 'the agents finish'
}

/** "1m", "12m", "1h 05m", "2d 3h": how long an agent has been working. */
export function elapsed(since: number, now: number): string {
  const minutes = Math.max(1, Math.floor((now / 1000 - since) / 60))
  if (minutes < 60) return `${minutes}m`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ${String(minutes % 60).padStart(2, '0')}m`
  return `${Math.floor(hours / 24)}d ${hours % 24}h`
}

export function agentStatus(agent: Agent, now: number): string {
  if (agent.state !== 'working') return 'Waiting for you'
  return `Working · ${elapsed(agent.since, now)}${agent.paused ? ' · paused' : ''}`
}

export type ConnectionAction = 'connect' | 'disconnect' | 'reconnect'

/** The one button a connection row offers. */
export function connectionAction(connection: AgentConnection): ConnectionAction {
  if (connection.status === 'connected') return 'disconnect'
  if (connection.status === 'stale') return 'reconnect'
  return 'connect'
}

export function connectionStatus(connection: AgentConnection): string {
  switch (connection.status) {
    case 'connected': return connection.error ?? 'Connected'
    case 'disconnected': return 'Not connected'
    case 'stale': return 'Wick was moved'
    case 'missing': return 'Not installed'
    case 'error': return connection.error ?? `Could not read ${connection.file}.`
  }
}

/** Codex also gets trust entries in config.toml next to its hooks file; null for Claude Code. */
export function trustFile(connection: AgentConnection): string | null {
  return connection.agent === 'codex' ? connection.file.replace(/[^/\\]+$/, 'config.toml') : null
}
