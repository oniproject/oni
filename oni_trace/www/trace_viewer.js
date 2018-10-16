let hex_view = document.getElementById('hex')
let file_input = document.getElementById('file')

file_input.onchange = function () {
}

const TAB = 9
const SPECIAL = 0x2400
const DEL = '␡'
const HEX_PREFIX = 4
const HEX_DIV = '│' // │
const HEX_EOF = '␣'
const TRI_R = '▶'
const TRI_D = '▼'


function ascii_print(c, replacement) {
	let s
	if (c < 32 || c >= 127) {
		s = replacement || '.'
	} else {
		s = String.fromCharCode(c)
	}
		/*
	if (c > 127) {
		s = '.'
	} else if (c == TAB) {
		s = ' '
	} else if (c == 127) {
		s = DEL
	} else if (c < 32) {
		s = String.fromCharCode(c + SPECIAL)
	} else {
		s = String.fromCharCode(c)
	}
	*/
	return s
}

function hexdump(array, split) {
	split = split || 16

	let s = ''

	let lines = (array.length / split) | 0
	if (array.length % split !== 0) lines += 1

	for (let line = 0; line < lines; line++) {
		let start = line * split
		let sub = array.subarray(start, start + split)

		s += start.toString(16).toUpperCase().padStart(HEX_PREFIX, '0')
		s += HEX_DIV

		for (let i = 0; i < split; i++) {
			let v = sub[i]
			if (i !== 0) {
				s += ' ';
			}
			if (i == 8) s += ' '
			if (v !== undefined) {
				s += v.toString(16).toUpperCase().padStart(2, '0')
			} else {
				s += '  '
			}
		}

		s += HEX_DIV

		for (let i = 0; i < split; i++) {
			let v = sub[i]
			if (v !== undefined) {
				s += ascii_print(v, String.fromCharCode(0xFFFD))
			} else {
				s += HEX_EOF
			}
		}

		s += '\n'
	}

	return s
}

let proc = [
	{
		expand: false,
		pid: 123,
		name: 'Client 1',
		threads: [
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th2', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th3', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th4', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
		]
	},
	{
		expand: true,
		pid: 333,
		name: 'Server',
		threads: [
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th2', tid: 32, tracks: [1, 2, 3, 4] },
		]
	},
	{
		expand: false,
		pid: 213,
		name: 'Client 2',
		threads: [
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th2', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th3', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th4', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
		]
	},
	{
		expand: true,
		pid: 333,
		name: 'Client 3',
		threads: [
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th2', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th3', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th4', tid: 32, tracks: [1, 2, 3, 4] },
			{ name: 'th1', tid: 32, tracks: [1, 2, 3, 4] },
		]
	},
]

const app = new Vue({
	el: '#app',
	data: {
		hex: '',
		tab: 'hex',
		proc: proc,
	},
	methods: {
		load_file(event) {
			let file = new FileReader()
			file.readAsArrayBuffer(event.target.files[0])
			file.onload = () => {
				let array = new Uint8Array(file.result)
				this.hex = hexdump(array, 8)
			}
		},
		toggle_tab(name) {
			if (this.tab == name) {
				this.tab = ''
			} else {
				this.tab = name
			}
		},
	},
})
