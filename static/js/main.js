'use strict'

//let ws_host = window.location.host

const game = new Game(`ws://${WS_HOST}/ws`)
document.body.appendChild(game.view)

game.ui = new Vue(UI)
game.client = new Client(game.ws)

game.ui.$on('focus', () => game.listener.stop_listening())
game.ui.$on('blur', () => game.listener.listen())

game.ui.$on('submit', () => {
	let input = document.getElementById('msg')
	game.ws.send(input.value)
	input.value = ''
})

game.ws.onerror = function (event) {
	console.error('ws', event)
}

game.ws.onmessage = function (event) {
	let messages = document.getElementById('messages')
	if (event.data instanceof ArrayBuffer) {
		let data = CBOR.decode(event.data)
		//console.log(data)
		switch (data[0]) {
		case 'W':
			let entity_id = data[1].id
			let last_processed_input = data[2]
			let states = data[3]

			game.client.entity_id = entity_id

			for (let i = 0; i < states.length; i++) {
				let state = states[i]
				game.client.process_server_message({
					entity_id: state[0].id,
					position: state[1],
					velocity: state[2],
				}, last_processed_input)
			}

			let pos = states.find(e => e[0].id === entity_id)[1]

			//let e = game.client.entities[entity_id]

			//console.log(entity_id)

			game.pos.x = pos[0]
			game.pos.y = pos[1]
			break
		}
	} else {
		game.ui.messages.push(event.data)
	}
}

let BAT = new Bat()


let bats = [
	new Bat(),
	new Bat(),
	new Bat(),
	new Bat(),
	new Bat(),
	new Bat(),
]

bats.forEach(b => b.visible = false)

PIXI.loader
	.add('bunny', 'bunny.png')
	.load((loader, resources) => {
		const bunny = new PIXI.Sprite(resources.bunny.texture)

		let w2 = game.renderer.width / 2
		let h2 = game.renderer.height / 2

		bunny.x = game.pos.x + w2
		bunny.y = game.pos.y + h2

		BAT.x = w2
		BAT.y = h2

		bunny.anchor.x = 0.5
		bunny.anchor.y = 0.5

		let graphics = new PIXI.Graphics()

		bats.forEach(b => game.stage.addChild(b))

		game.stage.addChild(bunny)
		game.stage.addChild(BAT)

		game.stage.addChild(graphics)

		game.ticker.add((dt) => {
			game.client.update()

			bunny.rotation += 0.1 * dt

			game.send_vel()

			let w2 = game.renderer.width / 2
			let h2 = game.renderer.height / 2

			BAT.x = (game.pos.x + w2) | 0
			BAT.y = (game.pos.y + h2) | 0

			graphics.x = w2 | 0
			graphics.y = h2 | 0

			if (true) {
				graphics.clear()

				graphics.beginFill(0x00FF00)
				for (let k in game.client.entities) {
					let e = game.client.entities[k]
					graphics.drawCircle(e.position[0], e.position[1], 2)
				}
				graphics.endFill()
			}

			BAT.vel_dir4(game.vel.x, game.vel.y)
		})
	})

const js_memory = new WebAssembly.Memory({ initial: 256, maximum: 256 })
const js_buffer = new Uint8Array(js_memory.buffer)
var wasm_memory

let importObject = {
	env: {
		memory: js_memory,
		memoryBase: 0,
		table: new WebAssembly.Table({ initial: 0, maximum: 0, element: 'anyfunc' }),
		tableBase: 0,

		print(ptr, len) {
			let str = from_wasm_str(ptr, len)
			console.log(ptr, len, str)
		},
	},
}

WebAssembly.instantiateStreaming(fetch('hello.wasm'), importObject)
.then(obj => {
	console.log('hell', obj)
	wasm_memory = obj.instance.exports.memory

	obj.instance.exports.print_hello()
})
.catch(error => {
	console.log('There has been a problem with your fetch operation: ' + error.message);
})

function from_wasm_str(offset, len) {
	let buffer = new Uint8Array(wasm_memory.buffer)
	let s = ''
	let end = offset + len
	for (let i = offset; i !== end; i++) {
		s += String.fromCharCode(buffer[i])
	}
	return s
}

function parse_utf8(h, p) {
	let s = ''
	for (let i = p; h[i]; i++) {
		s += String.fromCharCode(h[i])
	}
	return s
}

	/*
function toJsStr(buffer, offset) {
	let s = ''
	for (;; offset++) {
		let b = buffer[offset]
		if (b === 0) {
			return s
		}
		s += String.fromCharCode(b)
	}
}

function toCStr(buffer, str) {
	let offset = exports._getBuffer()
	for (let i = 0; i < str.length; ++i) {
		buffer[offset] = str.charCodeAt(i)
		offset += 1
	}
	buffer[offset] = 0
}

let ws
const buffer = new Uint8Array(memory.buffer)
let exports

function wsConnect(offset) {
	ws = new WebSocket(toJsStr(offset))
	ws.onopen = function() {
		console.log("Socket is open")
		exports._wsOnOpen()
	}

	ws.onmessage = function (evt) {
		var msg = evt.data
		console.log("Message is received...")
		toCStr(msg)
		exports._wsOnMessage()
	}
}

function wsClose() {
	ws.close()
}

function wsSend(offset) {
	ws.send(toJsStr(offset))
}

fetch('ws_test.wasm').then(response => 
	response.arrayBuffer()
).then(bytes => {
	let imports = {}
	imports.env = {}
	imports.env.memory = memory
	imports.env.memoryBase = 0
	imports.env.table = new WebAssembly.Table({ initial: 0, maximum: 0, element: 'anyfunc' })
	imports.env.tableBase = 0
	imports.env._print = print
	imports.env._wsConnect = wsConnect
	imports.env._wsSend = wsSend
	imports.env._wsClose = wsClose

	return WebAssembly.instantiate(bytes, imports)
}).then(module => {
	exports = module.instance.exports 
	exports._test()
})
*/
