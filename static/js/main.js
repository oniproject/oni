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

game.ws.onmessage = function (event) {
	let messages = document.getElementById('messages')
	if (event.data instanceof ArrayBuffer) {
		let data = CBOR.decode(event.data)
		//console.log(data)
		switch (data[0]) {
		case 'W':
			let entity_id = data[1]
			let last_processed_input = data[2]
			let states = data[3]

			game.client.entity_id = entity_id

			for (let i = 0; i < states.length; i++) {
				let state = states[i]
				game.client.process_server_message({
					entity_id: state[0],
					position: state[1],
					velocity: state[2],
				}, last_processed_input)
			}

			let pos = states.find(e => e[0] === entity_id)[1]

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
