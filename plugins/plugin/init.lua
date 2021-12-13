function onLoad()
	
end

function onUnload()
	
end

server.addEvent("message", function(session_id, message)
	
end)

Session = {
	id=-1,
	requested_grid=false
}

sessions = {}

function Session:new (o)
  o = o or {}
	setmetatable(o, self)
  self.__index = self
  return o
end


--server.addEvent("tick", function()
--end)

server.addEvent("command", function(session_id, command)
	if command == "grid" then
		local session = sessions[session_id]
		session.requested_grid = true
		server.userSendMessage(session_id, "Click where to draw grid")
	end
end)

server.addEvent("user_mouse_down", function(session_id)
	local session = sessions[session_id]
	if session.requested_grid then
		session.requested_grid = false
		local grid_x, grid_y = server.userGetPosition(session_id)
		
		for y=0, 500-1, 1 do
			for x=0, 500-1, 1 do
				if x%10 < 1 or y%10 < 1 then
					server.mapSetPixel(grid_x+x, grid_y+y, 180, 180, 180)
				end
			end
		end
	end
end)

server.addEvent("user_join", function(session_id) 
	local session = Session:new()
	session.id = session_id

	sessions[session_id] = session
end)

server.addEvent("user_leave", function(session_id)
	if drawing_session_id == session_id then
		drawing = false
	end
	
	table.remove(sessions, session_id)
end)


