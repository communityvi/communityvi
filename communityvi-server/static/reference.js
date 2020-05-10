'use strict';

let webSocket = null;
const websocketURL = `ws://${window.location.host}/ws`;

let lastSentGetReferenceTime = null;
let referenceTimeOffset = null;
let referenceTime = null;
const referenceTimeDisplay = document.getElementById('reference_time');
let nextRequestId = 0;

// requestId -> {"resolve": function, "reject": function, "requestType": string}
let pendingResponses = {}

let playerMode = 'fake';
let selectedMediumFile = null;
let mediumLength = null;
let playbackState = {
	type: 'paused',
	startTime: 0,
	position: 0,
};
const mediumNameLabel = document.getElementById('medium_name');
const mediumLengthLabel = document.getElementById('medium_length');
const playerPositionLabel = document.getElementById('player_position');
const playingMedium = document.getElementById('playing_medium');
const playerSelect = document.getElementById('player_select');
playerMode = playerSelect.value;
const playerReal = document.getElementById('player_real');
playerSelect.onchange = function () {
	playerMode = playerSelect.value;
}
const insertMediumInput = document.getElementById('insert_medium_input');
insertMediumInput.onchange = function () {
	if (insertMediumInput.files.length !== 1) {
		return;
	}

	selectedMediumFile = insertMediumInput.files[0];
	mediumNameLabel.textContent = selectedMediumFile.name;
	playerReal.src = URL.createObjectURL(selectedMediumFile);

	playerReal.load();
}
const insertMediumButton = document.getElementById('insert_medium');
insertMediumButton.onclick = function () {
	if (webSocket === null) {
		return;
	}

	if (playerMode === 'real') {
		playerPositionLabel.hidden = true;
		playerReal.hidden = false;

		insertMediumInput.click();

		return;
	}

	const name = prompt('What should the fake medium be called?', 'Birdman');
	if (name === null) {
		return;
	}

	const lengthPrompt = prompt('How long in minutes should the fake medium be?', '116');
	const length = Number.parseInt(lengthPrompt) * 60 * 1000;
	if (lengthPrompt === null || Number.isNaN(length)) {
		return;
	}

	sendMessage({type: 'insert_medium', name: name, length_in_milliseconds: length})
		.catch((error) => {
			console.error(`Failed to insert fake medium. ${error}`);
		});
};
playerReal.addEventListener('loadeddata', function () {
	playingMedium.style.height = `${this.videoHeight}px`;
	playingMedium.style.width = `${this.videoWidth}px`;
	playerPositionSlider.style.width = `${this.videoWidth}px`;

	sendMessage({type: 'insert_medium', name: mediumNameLabel.textContent, length_in_milliseconds: Math.round(this.duration * 1000)}).catch((error) => {
		console.error(`Failed to insert medium. ${error}`);
	});
});
const playPauseButton = document.getElementById('play_pause');
playPauseButton.onclick = function () {
	if (mediumLength === null) {
		return;
	}

	switch (playbackState.type) {
		case 'playing': {
			playbackState.type = 'paused';
			playbackState.position = calculateReferenceTime() - playbackState.startTime;
			sendMessage({type: 'pause', position_in_milliseconds: Math.round(playbackState.position), skipped: false})
				.catch((error) => {
					// TODO: Use this information to pause again?
					console.log(`Failed to pause video. ${error}`);
				});

			break;
		}

		case 'paused': {
			playbackState.type = 'playing';
			playbackState.startTime = calculateReferenceTime() - playbackState.position;
			sendMessage({type: 'play', start_time_in_milliseconds: Math.round(playbackState.startTime), skipped: false})
				.catch((error) => {
					// TODO: Use this information to play again?
					console.log(`Failed to play video. ${error}`);
				});

			break;
		}
	}

	updatePlayer();
};
const rewind10SecondsButton = document.getElementById('rewind_10');
rewind10SecondsButton.onclick = function () {
	skip(playerPositionSlider.valueAsNumber - (10 * 1000));
};
const forward10SecondsButton = document.getElementById('forward_10');
forward10SecondsButton.onclick = function () {
	skip(playerPositionSlider.valueAsNumber + (10 * 1000));
};
let sliderIsBeingDragged = false;
const playerPositionSlider = document.getElementById('player_position_slider');
playerPositionSlider.onmousedown = function () {
	sliderIsBeingDragged = true;
}
playerPositionSlider.onmouseup = function () {
	sliderIsBeingDragged = false;
}
playerPositionSlider.onchange = function () {
	skip(playerPositionSlider.value);
}

function skip(position) {
	if (mediumLength === null) {
		return;
	}

	switch (playbackState.type) {
		case 'playing': {
			const referenceTime = calculateReferenceTime();
			const startTime = referenceTime - position;
			if (startTime < (referenceTime - mediumLength)) {
				playbackState.startTime = referenceTime - mediumLength;
			} else if (startTime > referenceTime) {
				playbackState.startTime = referenceTime;
			} else {
				playbackState.startTime = startTime;
			}

			sendMessage({type: 'play', start_time_in_milliseconds: Math.round(playbackState.startTime), skipped: true})
				.catch((error) => {
					console.error(`Failed to skip in playing video. ${error}`);
				});
			break;
		}

		case 'paused': {
			if (position > mediumLength) {
				playbackState.position = mediumLength;
			} else if (position < 0) {
				playbackState.position = 0;
			} else {
				playbackState.position = position;
			}

			sendMessage({type: 'pause', position_in_milliseconds: Math.round(playbackState.position), skipped: true})
				.catch((error) => {
					console.error(`Failed to skip in paused video. ${error}`)
				});
			break;
		}
	}

	updatePlayer();
}

setupButtonPressOnEnter();

function setupButtonPressOnEnter() {
	const nameField = document.getElementById('name');
	const registerButton = document.getElementById('register');
	const messageField = document.getElementById('message');
	const sendButton = document.getElementById('send_message');

	addActionToButtonAndTextField(registerButton, nameField, registerClient);
	addActionToButtonAndTextField(sendButton, messageField, sendChatMessage);
}

function addActionToButtonAndTextField(button, textField, action) {
	// https://stackoverflow.com/questions/12955222/how-to-trigger-html-button-when-you-press-enter-in-textbox
	textField.addEventListener('keyup', event => {
		if (event.key !== 'Enter') {
			return;
		}

		action();

		event.preventDefault();
	});

	button.onclick = action;
}

function registerClient() {
	const nameField = document.getElementById('name');
	const registerButton = document.getElementById('register');
	const name = nameField.value;

	// disable to prevent further registrations
	nameField.disabled = true;
	registerButton.disabled = true;

	if (webSocket !== null) {
		webSocket.close();
	}

	webSocket = new WebSocket(websocketURL);
	webSocket.onopen = function (event) {
		console.log('Socket open.', event);
		const registerMessage = {
			type: 'register',
			name: name,
		};
		sendMessage(registerMessage)
			.then((response) => {
				const idField = document.getElementById('client_id');
				idField.innerText = response.id;

				const currentMedium = response.currentMedium;
				if (currentMedium !== null) {
					mediumLength = currentMedium.length_in_milliseconds;

					mediumNameLabel.textContent = currentMedium.name;
					mediumLengthLabel.textContent = Math.round(currentMedium.length_in_milliseconds / 1000 / 60).toString();

					playerPositionSlider.max = currentMedium.length_in_milliseconds;

					playbackState.type = currentMedium.playback_state.type;
					switch (currentMedium.playback_state.type) {
						case 'playing': {
							playbackState.startTime = currentMedium.playback_state.start_time_in_milliseconds;
							break;
						}

						case 'paused': {
							playbackState.position = currentMedium.playback_state.position_in_milliseconds;
							break;
						}
					}

					updatePlayer();
				}

				// start counter management
				setInterval(updateApplicationState, 16);
				requestReferenceTime();
				setInterval(requestReferenceTime, 10000);
			})
			.catch((error) => console.error(`Registration failed. ${error}`));
	};

	webSocket.onmessage = function (messageEvent) {
		console.log('Received message.', messageEvent);
		const messageData = messageEvent.data;
		const message = JSON.parse(messageData);
		handleMessage(message, messageEvent);
	};

	webSocket.onclose = function (event) {
		console.log('Socket closed.', event);
	};

	webSocket.onerror = function (event) {
		console.error('Received websocket error.', event);
	};
}

const requestTypeToExpectedResponseType = {
	'register': 'hello',
	'chat': 'success',
	'insert_medium': 'success',
	'get_reference_time': 'reference_time',
	'play': 'success',
	'pause': 'success',
};

function handleMessage(websocketMessage, messageEvent) {
	const message = websocketMessage.message;
	const requestID = websocketMessage.request_id;

	switch (websocketMessage.type) {
		case 'broadcast':
			handleBroadcast(message, messageEvent);
			break;

		case 'success': {
			const pendingPromise = pendingResponses[requestID];

			if (requestTypeToExpectedResponseType[pendingPromise.requestType] !== message.type) {
				pendingPromise.reject(`Invalid response type. Expected ${pendingPromise.requestType} got ${message.type}`);
				return;
			}

			switch (message.type) {
				case 'hello':
					pendingPromise.resolve({currentMedium: message.current_medium, id: message.id});
					break;

				case 'reference_time':
					pendingPromise.resolve({referenceTime: message.milliseconds, timeStamp: messageEvent.timeStamp});
					break;

				case 'success':
					pendingPromise.resolve();
					break;

				default:
					console.error(`Received invalid message ${JSON.stringify(websocketMessage)}`);
			}
			break;
		}

		case 'error': {
			if (requestID == null) {
				alert(`Error response without request_id. ${JSON.stringify(websocketMessage)}`)
				return;
			}

			const pendingPromise = pendingResponses[requestID];
			const errorMessage = message.message;
			pendingPromise.reject(`${errorMessage.error}: ${errorMessage.message}`);
			break;
		}

		default: {
			console.error(`Received invalid message: ${JSON.stringify(websocketMessage)}`);
		}
	}
}

function handleBroadcast(message, messageEvent) {
	switch (message.type) {
		case 'client_joined': {
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} joined the room.`);
			break;
		}

		case 'client_left': {
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} left the room.`);
			break;
		}

		case 'chat': {
			displayChatMessage(message.sender_id, message.sender_name, message.message);
			break;
		}

		case 'medium_state_changed': {
			if (mediumNameLabel.textContent !== message.medium.name) {
				displayChatMessage(message.changed_by_id, message.changed_by_name, `<<< inserted "${message.medium.name}" >>>`);
			}

			mediumNameLabel.textContent = message.medium.name;
			mediumLengthLabel.textContent = Math.round(message.medium.length_in_milliseconds / 1000 / 60).toString();
			mediumLength = message.medium.length_in_milliseconds;
			playerPositionSlider.max = mediumLength;

			playbackState.type = message.medium.playback_state.type;
			switch (playbackState.type) {
				case 'playing': {
					playbackState.startTime = message.medium.playback_state.start_time_in_milliseconds;

					if (message.medium.playback_skipped === true) {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) skipped.`);
					} else {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) started playback.`);
					}

					break;
				}
				case 'paused': {
					playbackState.position = message.medium.playback_state.position_in_milliseconds;

					if (message.medium.playback_skipped === true) {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) skipped.`);
					} else {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) paused playback.`);
					}

					break;
				}
			}

			updatePlayer();

			break;
		}

		default: {
			console.error(`UNKNOWN MESSAGE TYPE: '${message.type}'!`);
			break;
		}
	}
}

async function sendChatMessage() {
	const messageField = document.getElementById('message');
	const message = messageField.value;

	const chatMessage = {
		type: "chat",
		message: message,
	};
	await sendMessage(chatMessage);

	messageField.value = '';
}

async function sendMessage(message) {
	return new Promise((resolve, reject) => {
		message['request_id'] = nextRequestId;
		pendingResponses[nextRequestId] = {
			resolve: resolve,
			reject: reject,
			requestType: message.type,
		}
		nextRequestId++;
		webSocket.send(JSON.stringify(message));
	})
}

function displayChatMessage(id, name, message) {
	const chat = document.getElementById('chat');
	const row = chat.insertRow();
	row.insertCell().appendChild(document.createTextNode(id));
	row.insertCell().appendChild(document.createTextNode(name));
	row.insertCell().appendChild(document.createTextNode(message));
}

function updateApplicationState() {
	if (referenceTime == null) {
		return;
	}

	referenceTime = calculateReferenceTime();
	referenceTimeDisplay.innerText = `${Math.round(referenceTime)} ms`;

	if (playbackState.type === 'playing') {
	    const currentPosition = referenceTime - playbackState.startTime;

		if (currentPosition >= mediumLength) {
			sendMessage({type: 'pause', position_in_milliseconds: mediumLength, skipped: false})
				.catch((error) => {
					console.error(`Failed to pause at the end of the video. ${error}`)
				});
			playbackState.type = 'paused';
			playbackState.position = mediumLength;
		}

		updatePlayer();
	}
}

function updatePlayer() {
	switch (playbackState.type) {
		case 'playing': {
			const position = calculateReferenceTime() - playbackState.startTime;
			if (sliderIsBeingDragged === false) {
				playerPositionSlider.value = Math.round(position);
			}

			if (playerMode === 'real') {
				if (Math.abs(((playerReal.currentTime * 1000) - position)) > 1000) {
					playerReal.currentTime = Math.round(position) / 1000;
				}
				if (playerReal.paused) {
					playerReal.play();
				}
			} else {
				playerPositionLabel.textContent = (Math.round(position) / 1000).toString();
			}

			playPauseButton.innerHTML = '&#9208;';
			break;
		}
		case 'paused': {
			if (sliderIsBeingDragged === false) {
				playerPositionSlider.value = Math.round(playbackState.position);
			}

			if (playerMode === 'real') {
				if (Math.abs(((playerReal.currentTime * 1000) - playbackState.position)) > 1000) {
					playerReal.currentTime = Math.round(playbackState.position) / 1000;
				}
				if (!playerReal.paused) {
					playerReal.pause();
				}
			} else {
				playerPositionLabel.textContent = (Math.round(playbackState.position) / 1000).toString();
			}

			playPauseButton.innerHTML = '&#9654;';
			break;
		}
	}
}

function calculateReferenceTime() {
	return performance.now() + referenceTimeOffset;
}

function requestReferenceTime() {
	if (webSocket == null) {
		return;
	}

	lastSentGetReferenceTime = performance.now();
	sendMessage({type: "get_reference_time"}).then((response) => {
		const elapsed = response.timeStamp - lastSentGetReferenceTime;
		const serverReferenceTime = response.referenceTime;
		const now = performance.now();
		if (referenceTimeOffset == null) {
			referenceTimeOffset = serverReferenceTime - (now - elapsed / 2);
		} else {
			const localReferenceTime = (now - elapsed / 2) + referenceTimeOffset;
			referenceTimeOffset += serverReferenceTime - localReferenceTime;
		}
		console.log(`offset: ${referenceTimeOffset}`);

		if (referenceTime == null) {
			referenceTime = now + referenceTimeOffset;
		}
	}).catch((error) => {
		console.error(`Failed to get reference time. ${error}`)
	});
}
