Tick = 0.0

-- Tick function that will be executed every logic tick
function onTick()
	Tick = Tick + 1
	x = input.getNumber(1)
	y = input.getNumber(2)
	z = input.getNumber(3)
	rx = input.getNumber(4)
	ry = input.getNumber(5)
	rz = input.getNumber(6)
	ry_old = input.getNumber(7)
    if Tick % 4 == 0 then
		async.httpGet(14514, string.format("/?tick=%f&x=%f&y=%f&z=%f&rx=%f&ry=%f&rz=%f&ry_old=%f", Tick, x, y, z, rx, ry, rz, ry_old))
	end
end
