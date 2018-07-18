'use strict'

const SERVER_UPDATE_RATE = 10.0

// An Entity in the world.
class Entity {
	constructor(id) {
		this.entity_id = id
		this.position = [0, 0]
		this.velocity = [2, 2] // units/s
		this.position_buffer = []
	}

	// Apply user's input to this entity.
	apply_input(input) {
		//throw 'panic'
		this.position[0] += this.velocity[0] * input.press_time
		this.position[1] += this.velocity[1] * input.press_time
	}

	interpolate(render_timestamp) {
		// Find the two authoritative positions surrounding the rendering timestamp.
		let buffer = this.position_buffer

		// Drop older positions.
		while (buffer.length >= 2 && buffer[1].time <= render_timestamp) {
			buffer.shift()
		}

		// Interpolate between the two surrounding authoritative positions.
		if (buffer.length >= 2 && buffer[0].time <= render_timestamp && render_timestamp <= buffer[1].time) {
			let p0 = buffer[0].position
			let p1 = buffer[1].position
			let t0 = buffer[0].time
			let t1 = buffer[1].time

			this.position[0] = p0[0] + (p1[0] - p0[0]) * (render_timestamp - t0) / (t1 - t0)
			this.position[1] = p0[1] + (p1[1] - p0[1]) * (render_timestamp - t0) / (t1 - t0)
		}
	}
}

// The Client.
class Client {
	constructor(network) {
		// Local representation of the entities.
		this.entities = {}

		// Input state.
		this.key_left = false
		this.key_right = false

		// Simulated network connection.
		//this.network = new LagNetwork()
		this.network = network
		this.server = null
		this.lag = 0

		// Unique ID of our entity.
		// Assigned by Server on connection.
		this.entity_id = null

		// Data needed for reconciliation.
		this.client_side_prediction = true
		this.server_reconciliation = false
		this.input_sequence_number = 0
		this.pending_inputs = []

		// Entity interpolation toggle.
		this.entity_interpolation = true

		// UI.
		//this.canvas = canvas
		//this.status = status

		// Update rate.
		//this.setUpdateRate(50)
	}

	// Update Client state.
	update() {
		// Listen to the server.
		//this.process_server_messages()

		if (this.entity_id === null) {
			return // Not connected yet.
		}

		// Process inputs.
		// FIXME this.process_inputs()

		// Interpolate other entities.
		if (this.entity_interpolation) {
			this.interpolate_entities()
		}

		// Show some info.
		//console.log("Non-acknowledged inputs: ", this.pending_inputs.length)
	}

	// Get inputs and send them to the server.
	// If enabled, do client-side prediction.
	process_inputs() {
		// Compute delta time since last update.
		let now_ts = +new Date()
		let last_ts = this.last_ts || now_ts
		this.last_ts = now_ts

		let dt_sec = (now_ts - last_ts) / 1000.0

		// Package player's input.
		let press_time
		if (this.key_right) {
			press_time = dt_sec
		} else if (this.key_left) {
			press_time = -dt_sec
		} else {
			// Nothing interesting happened.
			return
		}

		// Send the input to the server.
		this.server.network.send(this.lag, {
			press_time: press_time,
			input_sequence_number: this.input_sequence_number++,
			entity_id: this.entity_id,
		})

		// Do client-side prediction.
		if (this.client_side_prediction) {
			this.entities[this.entity_id].apply_input(input)
		}

		// Save this input for later reconciliation.
		this.pending_inputs.push(input)
	}


	// Process all messages from the server, i.e. world updates.
	// If enabled, do server reconciliation.
	process_server_messages() {
		while (true) {
			let message = this.network.receive()
			if (!message) {
				break
			}

			process_world_state_list(message)

		}
	}

	process_world_state_list(message) {
		// World state is a list of entity states.
		for (let i = 0; i < message.length; i++) {
			let state = message[i]
			this.process_server_message(state)
		}
	}

	// Process all messages from the server, i.e. world updates.
	// If enabled, do server reconciliation.
	process_server_message(state, last_processed_input) {
		// If this is the first time we see this entity, create a local representation.
		if (!this.entities[state.entity_id]) {
			this.entities[state.entity_id] = new Entity(state.entity_id)
		}

		let entity = this.entities[state.entity_id]

		if (state.entity_id === this.entity_id) {
			// Received the authoritative position of this client's entity.
			entity.position = state.position

			if (this.server_reconciliation) {
				// Server Reconciliation.
				// Re-apply all the inputs not yet processed by
				// the server.
				let j = 0
				while (j < this.pending_inputs.length) {
					let input = this.pending_inputs[j]
					if (input.input_sequence_number <= last_processed_input) {
						// Already processed.
						// Its effect is already taken into account into the world update
						// we just got, so we can drop it.
						this.pending_inputs.splice(j, 1)
					} else {
						// Not processed by the server yet. Re-apply it.
						entity.applyInput(input)
						j++
					}
				}
			} else {
				// Reconciliation is disabled, so drop all the saved inputs.
				this.pending_inputs = []
			}
		} else {
			// Received the position of an entity other than this client's.

			if (!this.entity_interpolation) {
				// Entity interpolation is disabled - just accept the server's position.
				entity.position = state.position
			} else {
				// Add it to the position buffer.
				let timestamp = +new Date()
				entity.position_buffer.push({
					time: timestamp,
					position: state.position,
				})
			}
		}
	}

	interpolate_entities() {
		// Compute render timestamp.
		let now = +new Date()
		let render_timestamp = now - (1000.0 / SERVER_UPDATE_RATE)

		for (let k in this.entities) {
			let entity = this.entities[k]

			// No point in interpolating this client's entity.
			if (entity.entity_id !== this.entity_id) {
				entity.interpolate(render_timestamp)
			}
		}
	}
}
