# communityvi
communityvi (name subject to change) is a backend and frontend for consuming media together over the network, inspired by [syncplay](https://syncplay.pl) and togethertube (may it rest in peace).

It is currently still under development. We're lacking mostly in frontend design, UX and layout.

## TL;DR: I just want to try it out

Great! Just follow the server and frontend README's Getting Started sections and you'll arrive at a working system in minutes.
This is also an excellent way of getting started with development in general, after reading the passage about contributing down bwlow.

## Vision
We want to provide a platform that allows anyone to consume media (i.e. watching movies, listening to music...) together without any technical expertise required.

### Goals
* Synchronizing playback state between multiple people.
* Providing a one-click solution that only requires a URL to participate.
* Allow synchronized playback of video/audio files stored locally on the device where a user is watching/listening. (all participants need a local copy)
* Allow playback of video/audio from streaming sites like YouTube, Vimeo, Soundcloud etc.
* Providing a backend API that allows for additional client implementations.
	* e.g. native desktop or mobile apps
	* e.g. WebExtensions that hook directly into third party streaming sites that don't allow being embedded
* Making the hosting of your own instance as easy as possible.
	* ideally just one file that you can execute and just works out of the box
* The backend should run at least on the major platforms: Windows, macOS and Linux
* The builtin frontend should run at least in the major browsers: Firefox, Chrome, Edge and Safari

### Non-Goals
* Video or Audio chat
	* Bundling an existing, working solution isn't completely out of question
* Distributing video or audio files
* Circumventing Ads played by video streaming sites or any form or DRM
* Supporting outdated versions of browsers or operating systems that don't get security updates anymore.

## Features
Features that are currently implemented:
* Rudimentary web frontend that allows watching video/audio from local files (as long as the browser supports the codec)
* Text chat
* Bundling the frontend files in the backend binary, so only a binary is required to use it.

## Roadmap
Although we have a first working prototype that can (and has) already been used to watch videos together, there's still a lot to do.

This roadmap provides our current, preliminary plan which is subject to change:
1. Persistence across backend restarts
2. Converting request/response style websocket requests to HTTP requests, see #233
3. Support streaming sites in the backend
	1. Provide a working YouTube implementation.
	2. Add support for at least one other streaming site
4. Support multiple rooms
5. Authentication and permission system

## Contributing
This is a passion project we (@winf34k and @FSMaxB) are developing in our free time, mostly in pair programming sessions.
Note that development is slow and can sometimes stagnate for months at a time.
The idea has been floating around in our heads since around 2013 and we've been working on this code base since early 2019 to get us to the point we're currently at.

Help is always appreciated. Note that we don't yet have written guidelines of what we value in code contributions. We are generally aiming for readable and well tested code though.

If you plan to contribute, please open an issue with your intentions first, to reduce potential waste of effort if it isn't in line with what we're trying to achieve.

There are several things we don't have a good answer for yet, if you do, feel free to open a GitHub issue:

### [The Name](https://www.youtube.com/watch?v=Fwq699GN-Y8&t=496s)
The current name "communityvi" is a combination of "community" and "TV" but it isn't a very good name.
Not only because we as project founders don't even get its spelling right most of the time.

A good name should not conflicting with anything else when googling, be easy to spell and remember and ideally describe what the project is about. We haven't found one yet.

### UX
Currently all we're doing in that area is using the project ourselves and fixing the most annoying issues.

### Design/Layout
Our focus so far has been on structuring the frontend code in a way that hopefully somebody with the necessary skills can come in and write SASS styles for it that work.
But for know there hasn't been any attempt on making the frontend look nice or structured, just on making it work.

We also don't have a logo design. There have been initial attempts but they don't really scale down well to small icon sizes.

### Frontend Expertise
We are new to frontend development and are doing the best we can, but having someone with actual experience in that area taking a look at what we've done would be greatly appreciated.

