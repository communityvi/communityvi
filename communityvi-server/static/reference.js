'use strict';

let webSocket = null;
let messageNumber = 0;
const websocketURL = `ws://${window.location.host}/ws`;

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
		handleMessage(message);
	};

	webSocket.onclose = function (event) {
		console.log('Socket closed.', event);
	};

	webSocket.onerror = function (event) {
		console.log('Received error.', event);
	};
}

function handleMessage(message) {
	const idField = document.getElementById('client_id');

	switch (message.type) {
		case 'hello':
			idField.innerText = message.id;
			break;

		case 'joined':
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} joined the room.`);
			break;

		case 'left':
			displayChatMessage('', 'Server', `User ${message.name} with id ${message.id} left the room.`);
			break;

		case 'chat':
			displayChatMessage(message.sender_id, message.sender_name, message.message);
			break;
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
