const MAX_CHAT_LEN = 50

const UI = {
	el: '#chat',
	data: {
		messages: [
		],
	},

	computed: {
		msg() {
			return this.messages.filter(m => m.length > 0).slice(-MAX_CHAT_LEN)
		},
	},
}
