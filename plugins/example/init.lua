function onLoad()
	print("Plugin load test")
end

function onUnload()
	print("Plugin unload test")
end

server.addEvent("message", function(session_id, message)
	
end)

server.addEvent("command", function(session_id, command)
	if command == "help" then
		server.chatBroadcast("User " .. server.userGetName(session_id) .. " called for help!")
		server.userSendMessage(session_id, "Secret message");
	end
end)

server.addEvent("user_join", function(session_id) 
	server.chatBroadcast("User "..server.userGetName(session_id).." joined.");
end)

server.addEvent("user_leave", function(session_id)
	server.chatBroadcast("User"..server.userGetName(session_id).." left.");
end)