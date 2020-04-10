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
			setInterval(displayCounter, 16);
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

function displayCounter() {
	if (referenceTime == null) {
		return;
	}

	referenceTime = performance.now() + referenceTimeOffset;
	referenceTimeDisplay.innerText = `${Math.round(referenceTime)} ms`;
}

function requestReferenceTime() {
	if (webSocket == null) {
		return;
	}

	lastSentGetReferenceTime = performance.now();
	sendMessage({type: "get_reference_time"});
}
