'use strict'

const BAT_TEXTURE = PIXI.BaseTexture.fromImage('bat.png')

class Bat extends PIXI.Container {
	constructor() {
		super()

		this._anim = {}

		let w = 96 / 3
		let h = 128 / 4
		let arr = ['↑', '←', '↓', '→']
		arr.forEach((key, y) => {
			let textures = [0, 1, 2, 1]
				.map((x) => new PIXI.Rectangle(x * w, y * h, w, h))
				.map((r) => new PIXI.Texture(BAT_TEXTURE, r))

			let sprite = new PIXI.extras.AnimatedSprite(textures)
			sprite.anchor.x = 0.5
			sprite.anchor.y = 1
			sprite.animationSpeed = 0.15
			sprite.play()
			this._anim[key] = sprite
			this.addChild(sprite)
		})

		this.direction = '↓'
	}

	get direction() {
		return this._direction
	}
	set direction (v) {
		switch (v) {
			case '↗': v = '↑'; break
			case '↘': v = '→'; break
			case '↙': v = '↓'; break
			case '↖': v = '←'; break
		}
		this._direction = v

		for(var k in this._anim) {
			this._anim[k].visible = k === v
		}
	}

	set_dir_num(num) {
		let x = (v / (360 / 8)) % 8
		if (x < 0) {
			x = 8 + x
		}
		let dir = '↑↗→↘↓↙←↖'
		this.direction = dir[x | 0]
	}
	vel_dir4(x, y) {
		if (x === 0.0 && y === 0.0) {
			return
		}

		let a = Math.atan2(x, y)
		const pi4 = Math.PI / 4

		if (a <= pi4 && a >= -pi4) {
			this.direction = '↓'

		} else if (a >= pi4 * 3 || a <= -pi4 * 3) {
			this.direction = '↑'

		} else if (a >= pi4) {
			this.direction = '→'
		} else if (a <= -pi4) {
			this.direction = '←'
		} else {
			throw 'wtf?'
		}
	}
}
