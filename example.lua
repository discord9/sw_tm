Tick = 0.0
-- Tick function that will be executed every logic tick
function onTick()
	Tick = Tick + 1
	power = input.getBool(1)
	height = input.getNumber(1)			 -- Read the first number from the script's composite input
	pitch = input.getNumber(2)
	roll = input.getNumber(3)
	set_height = input.getNumber(4)
	PitchTarget = input.getNumber(5)
	RollTarget = input.getNumber(6)
	if Tick % 4 == 0  and power then
		async.httpGet(14514, string.format("/?tick=%f&height=%f&pitch=%f&roll=%f&setHeight=%f&PitchTarget=%f&RollTarget=%f", Tick, height, pitch, roll, set_height, PitchTarget, RollTarget))
	end
end

-- Draw function that will be executed when this script renders to a screen
function onDraw()
	w = screen.getWidth()				  -- Get the screen's width and height
	h = screen.getHeight()					
	screen.setColor(0, 255, 0)			 -- Set draw color to green
	screen.drawCircleF(w / 2, h / 2, 30)   -- Draw a 30px radius circle in the center of the screen
end