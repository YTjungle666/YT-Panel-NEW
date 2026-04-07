const packDate = new Date().toISOString().slice(0, 10).replace(/-/g, '')

process.stdout.write(`${packDate}\n`)
