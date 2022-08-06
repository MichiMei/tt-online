
import React, { Component } from 'react';
import { w3cwebsocket as W3CWebSocket } from "websocket";

//const client = new W3CWebSocket('wss://coding-capricorn.de:8080');
const client = new W3CWebSocket('ws://localhost:8080');

class App extends Component {

  constructor(props) {
    super(props);
    this.state = {fastReadToken: ""};
  }

  currentStateId = 0;
  currentState = "None";

  sendLogin() {
    if (client.readyState === client.OPEN) {
      // TODO get real data
      const name = "mock_name"
      const type = "ClientLogin"
      const message_obj = {type:type, name: name}
      const message_str = JSON.stringify(message_obj)

      console.log("sendLogin(..): " + message_str)
      client.send(message_str)
    }
  }

  componentDidMount() {
    client.onopen = () => {
      console.log('componentDidMount(..): WebSocket Client Connected');
      this.sendLogin()
    };
    client.onmessage = (message) => {
      console.log("received: " + message.data.toString());
      this.handleMessage(message.data)
    };
  }

  handleMessage(message) {
    let json = JSON.parse(message);

    switch (json.type) {
      case "Disconnect":
        this.handleDisconnect(json);
        break;
      case "Update":
        this.handleUpdate(json);
        break;
      case "ChangeState":
        this.handleStateChange(json);
        break;
      default:
        console.warn("received bad message: type " + json.type + " is not supported");
        break;
    }
  }

  handleDisconnect(json) {
    let reason = json.reason;
    console.warn("backend closed connection: " + reason);
    // TODO send disconnect to backend
    // TODO close websocket
    // TODO ask user if should try reconnect
  }

  handleUpdate(json) {
    let stateId = json.state_id;
    let update = JSON.parse(json.content);

    if (this.currentStateId !== stateId) {
      console.warn("received outdated update: got " + stateId + ", expected " + this.currentStateId);
    } else if (this.currentState !== "ActivityFastRead") {
      console.warn("received update for unsupported state " + this.currentState);
    } else {
      console.log("backend sent update: " + update)
      clearTimeout(this.timerFastRead);

      let duration = update.duration;
      let token = update.token;

      this.setState({fastReadToken: token});
      this.timerFastRead = setTimeout(
          () => {
            this.setState({fastReadToken: ""});
          },
          duration
      );
    }
  }

  handleStateChange(json) {
    let stateId = json.state_id;
    let state = json.content;

    if (state === "None" || state === "ActivityFastRead") {
      this.currentStateId = stateId;
      this.currentState = state;
      console.log("changed state to " + state);
    } else {
      console.warn("received unsupported state change: " + state);
    }
  }

  render() {
    return (
        <div>
          <h1>
            <center>
              {
                this.state.fastReadToken
              }
            </center>
          </h1>
        </div>
    );
  }
}

export default App;

/*
import logo from './logo.svg';
import './App.css';

function App() {
  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.js</code> and save to reload.
        </p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
}

export default App;
*/