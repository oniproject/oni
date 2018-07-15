'use strict'

//let ws_host = window.location.host

const game = new Game(`ws://${WS_HOST}/ws`)
document.body.appendChild(game.view)

game.ui = new Vue(UI)

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
		for (let i = 0, l = data.length; i < l; i++) {
			let cmd = data[i]
			switch (cmd[0]) {
			case 'A':
				game.pos.x = cmd[1].pos[0]
				game.pos.y = cmd[1].pos[1]
				break
			}
		}
	} else {
		game.ui.messages.push(event.data)
	}
}

let BAT = new Bat()

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

		game.stage.addChild(bunny)
		game.stage.addChild(BAT)

		game.ticker.add((dt) => {
			bunny.rotation += 0.1 * dt

			game.send_vel()

			let w2 = game.renderer.width / 2
			let h2 = game.renderer.height / 2

			BAT.x = (game.pos.x + w2) | 0
			BAT.y = (game.pos.y + h2) | 0

			BAT.vel_dir4(game.vel.x, game.vel.y)
		})
	})
