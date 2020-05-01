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

let mediumLength = null;
let playbackState = {
	type: 'paused',
	startTime: 0,
	position: 0,
};
const mediumNameLabel = document.getElementById('medium_name');
const mediumLengthLabel = document.getElementById('medium_length');
const playerPositionLabel = document.getElementById('player_position');
const playerPositionSlider = document.getElementById('player_position_slider');
const insertMediumButton = document.getElementById('insert_medium');
insertMediumButton.onclick = function () {
	if (webSocket === null) {
		return;
	}

	const name = prompt('What should the fake medium be called?', 'Birdman');
	const length = Number.parseInt(prompt('How long in minutes should the fake medium be?', '116')) * 60 * 1000;
	sendMessage({type: 'insert_medium', name: name, length_in_milliseconds: length});
};
const playPauseButton = document.getElementById('play_pause');
playPauseButton.onclick = function () {
	if (mediumLength === null) {
		return;
	}

	if (playbackState.type === 'paused') {
		playbackState.type = 'playing';
		playbackState.startTime = calculateReferenceTime() - playbackState.position;

		sendMessage({type: 'play', start_time_in_milliseconds: Math.round(playbackState.startTime), skipped: false});
		playPauseButton.innerHTML = '&#9208;';
	} else if (playbackState.type === 'playing') {
		playbackState.type = 'paused';
		playbackState.position = calculateReferenceTime() - playbackState.startTime;
		console.log(`calc ${calculateReferenceTime()} start ${playbackState.startTime} POS ${playbackState.position}`);

		sendMessage({type: 'pause', position_in_milliseconds: Math.round(playbackState.position), skipped: false});
		playPauseButton.innerHTML = '&#9654;';
	}
};
const rewind10SecondsButton = document.getElementById('rewind_10');
const forward10SecondsButton = document.getElementById('forward_10');

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
		    mediumLengthLabel.textContent = message.length_in_milliseconds / 1000 / 60;

		    playbackState.type = 'paused';
		    playbackState.position = 0;

		    playerPositionSlider.max = message.length_in_milliseconds;
		    playerPositionSlider.value = 0;
		    playerPositionLabel.textContent = 0;

		    displayChatMessage(message.inserted_by_id, message.inserted_by_name, `<< inserted "${message.name}" >>`);

			mediumLength = message.length_in_milliseconds;

			break;
		}

		case 'playback_state_changed': {
		    playbackState.type = message.playback_state.type;
			if (message.playback_state.type === 'playing') {
				playbackState.startTime = message.playback_state.start_time_in_milliseconds;
				playPauseButton.innerHTML = '&#9208;';
				displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) started playback.`);
			} else {
				playbackState.position = message.playback_state.position_in_milliseconds;
				playPauseButton.innerHTML = '&#9654;';
				displayChatMessage('', 'Server', `${message.changed_by_name} (Client ID: ${message.changed_by_id}) paused playback.`);
			}

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
	    	playbackState.position = mediumLength;
			playerPositionSlider.value = playbackState.position;
			playerPositionLabel.textContent = playbackState.position / 1000;
	    	playbackState.type = 'paused';
		} else {
			playerPositionSlider.value = Math.round(currentPosition);
			playerPositionLabel.textContent = currentPosition / 1000;
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
