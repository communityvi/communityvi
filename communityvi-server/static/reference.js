'use strict';

let webSocket = null;
let messageNumber = 0;
let lastSentPing = null;
const websocketURL = `ws://${window.location.host}/ws`;

const pingButton = document.getElementById('ping_button');
const pingDisplay = document.getElementById('ping_display');
pingButton.onclick = function () {
	if (webSocket == null) {
		return;
	}

	lastSentPing = performance.now();
	sendMessage({type: "ping"});
	pingButton.disabled = true;
};

let lastSentGetReferenceTime = null;
let referenceTimeOffset = null;
let referenceTime = null;
const referenceTimeDisplay = document.getElementById('reference_time');

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

	sendMessage({type: 'insert_medium', name: name, length_in_milliseconds: length});
};
playerReal.addEventListener('loadeddata', function () {
	playingMedium.style.height = `${this.videoHeight}px`;
	playingMedium.style.width = `${this.videoWidth}px`;
	playerPositionSlider.style.width = `${this.videoWidth}px`;

	sendMessage({type: 'insert_medium', name: mediumNameLabel.textContent, length_in_milliseconds: Math.round(this.duration * 1000)});
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
			sendMessage({type: 'pause', position_in_milliseconds: Math.round(playbackState.position), skipped: false});

			break;
		}

		case 'paused': {
			playbackState.type = 'playing';
			playbackState.startTime = calculateReferenceTime() - playbackState.position;
			sendMessage({type: 'play', start_time_in_milliseconds: Math.round(playbackState.startTime), skipped: false});

			break;
		}
	}

	updatePlayer();
};
const rewind10SecondsButton = document.getElementById('rewind_10');
rewind10SecondsButton.onclick = function () {
	skip(Number.parseInt(playerPositionSlider.value) - (10 * 1000));
};
const forward10SecondsButton = document.getElementById('forward_10');
forward10SecondsButton.onclick = function () {
	skip(Number.parseInt(playerPositionSlider.value) + (10 * 1000));
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
    skip(Number.parseInt(playerPositionSlider.value));
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

			sendMessage({type: 'play', start_time_in_milliseconds: Math.round(playbackState.startTime), skipped: true});
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

			sendMessage({type: 'pause', position_in_milliseconds: Math.round(playbackState.position), skipped: true});
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
		messageNumber = 0;
	}

	webSocket = new WebSocket(websocketURL);
	webSocket.onopen = function (event) {
		console.log('Socket open.', event);
		const registerMessage = {
			type: 'register',
			name: name,
		};
		sendMessage(registerMessage);
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
		console.log('Received error.', event);
	};
}

function handleMessage(message, messageEvent) {
	const idField = document.getElementById('client_id');

	switch (message.type) {
		case 'hello': {
			idField.innerText = message.id;

			if (message.current_medium !== null) {
				mediumLength = message.current_medium.length_in_milliseconds;

				mediumNameLabel.textContent = message.current_medium.name;
				mediumLengthLabel.textContent = Math.round(message.current_medium.length_in_milliseconds / 1000 / 60);

				playerPositionSlider.max = message.current_medium.length_in_milliseconds;

				playbackState.type = message.current_medium.playback_state.type;
				switch (message.current_medium.playback_state.type) {
					case 'playing': {
						playbackState.startTime = message.current_medium.playback_state.start_time_in_milliseconds;
						break;
					}

					case 'paused': {
						playbackState.position = message.current_medium.playback_state.position_in_milliseconds;
						break;
					}
				}

				updatePlayer();
			}

			// start counter management
			setInterval(updateApplicationState, 16);
			requestReferenceTime();
			setInterval(requestReferenceTime, 10000);
			break;
		}

		case 'joined': {
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} joined the room.`);
			break;
		}

		case 'left': {
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} left the room.`);
			break;
		}

		case 'chat': {
			displayChatMessage(message.sender_id, message.sender_name, message.message);
			break;
		}

		case 'pong': {
			const elapsed = messageEvent.timeStamp - lastSentPing;
			pingDisplay.innerText = `${Math.round(elapsed)} ms`;
			pingButton.disabled = false;
			break;
		}

		case 'reference_time': {
			const elapsed = messageEvent.timeStamp - lastSentGetReferenceTime;
			const serverReferenceTime = message.milliseconds;
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

			break;
		}

		case 'medium_inserted': {
		    mediumNameLabel.textContent = message.name;
		    mediumLengthLabel.textContent = Math.round(message.length_in_milliseconds / 1000 / 60);

		    playbackState.type = 'paused';
		    playbackState.position = 0;

		    playerPositionSlider.max = message.length_in_milliseconds;

		    displayChatMessage(message.inserted_by_id, message.inserted_by_name, `<< inserted "${message.name}" >>`);

			mediumLength = message.length_in_milliseconds;

			updatePlayer();

			break;
		}

		case 'playback_state_changed': {
		    playbackState.type = message.playback_state.type;
		    switch (message.playback_state.type) {
				case 'playing': {
					playbackState.startTime = message.playback_state.start_time_in_milliseconds;

					if (message.skipped === true) {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) skipped.`);
					} else {
						displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) started playback.`);
					}

					break;
				}
				case 'paused': {
					playbackState.position = message.playback_state.position_in_milliseconds;

					if (message.skipped === true) {
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

		case 'error': {
			console.error(`Received error message: [${message.error}] '${message.message}'`);
			break;
		}

		default: {
			console.error(`UNKNOWN MESSAGE TYPE: '${message.type}'!`);
			break;
		}
	}
}

function sendChatMessage() {
	const messageField = document.getElementById('message');
	const message = messageField.value;

	const chatMessage = {
		type: "chat",
		message: message,
	};
	sendMessage(chatMessage);

	messageField.value = '';
}

function sendMessage(message) {
	message.number = messageNumber;
	messageNumber++;
	webSocket.send(JSON.stringify(message));
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
	    	sendMessage({type: 'pause', position_in_milliseconds: mediumLength, skipped: false});
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
				playerPositionLabel.textContent = Math.round(position) / 1000;
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
				playerPositionLabel.textContent = Math.round(playbackState.position) / 1000;
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
	sendMessage({type: "get_reference_time"});
}
