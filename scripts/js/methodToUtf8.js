
const init = async () => {
    const args = process.argv.slice(2);
    console.log("String to Utf8", Buffer.from(args[0] || 'Heyy', 'utf8').toString('hex'))
}

init()
