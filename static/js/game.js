'use strict'

class Game extends PIXI.Application {
	constructor(addr) {
		super(window.innerWidth, window.innerHeight, {
			transparent: true,
		})

		this.ws = new WebSocket(addr)
		this.ws.binaryType = 'arraybuffer'

		this.listener = new window.keypress.Listener()

		this.vel = { x: 0, y: 0 }
		this.pos = { x: 0, y: 0 }

		this.init_keyboard()

		window.addEventListener('resize', ()=> {
			this.renderer.resize(window.innerWidth, window.innerHeight)
		})

		this.stage.interactive = true
		this.stage.hitArea = new PIXI.Rectangle(0, 0, 99999, 99999)

		this.touch = false

		this.stage.on('pointerdown', (event) => {
			this.touch = true
			this.turn_player(event)
		})
		this.stage.on('pointermove', (event) => {
			if (this.touch) this.turn_player(event)
		})
		let out = () => {
			this.touch = false
			this.vel.x = 0
			this.vel.y = 0
		}
		this.stage.on('pointerup', out)
		this.stage.on('pointerout', out)
		this.stage.on('pointercancel', out)
	}

	send_vel() {
		let data = CBOR.encode({
			press_time: 0.7,
			velocity: [this.vel.x, this.vel.y],
			input_sequence_number: 5,
		})
		this.ws.send(data)
	}

	init_keyboard() {
		this.listener.reset()
		this.listener.register_many([{
			keys: 'w',
			on_keydown() { this.vel.y = -1 },
			on_keyup()   { this.vel.y =  0 },
			this: this,
		}, {
			keys: 'a',
			on_keydown() { this.vel.x = -1 },
			on_keyup()   { this.vel.x =  0 },
			this: this,
		}, {
			keys: 's',
			on_keydown() { this.vel.y =  1 },
			on_keyup()   { this.vel.y =  0 },
			this: this,
		}, {
			keys: 'd',
			on_keydown() { this.vel.x =  1 },
			on_keyup()   { this.vel.x =  0 },
			this: this,
		}])
	}

	turn_player(event) {
		let w2 = this.renderer.width / 2
		let h2 = this.renderer.height / 2

		let pos = event.data.getLocalPosition(this.stage)

		let vx = pos.x - this.pos.x - w2
		let vy = pos.y - this.pos.y - h2

		let norm = Math.sqrt(vx * vx + vy * vy)
		if (norm != 0) {
			vx = vx / norm;
			vy = vy / norm;
		}
		this.vel.x = vx
		this.vel.y = vy
	}
}
