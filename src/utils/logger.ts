const canLogInDev = import.meta.env.DEV

export function logError(message: string, error?: unknown) {
  if (!canLogInDev)
    return

  if (error === undefined)
    console.error(message)
  else
    console.error(message, error)
}

export function logWarn(message: string, extra?: unknown) {
  if (!canLogInDev)
    return

  if (extra === undefined)
    console.warn(message)
  else
    console.warn(message, extra)
}
