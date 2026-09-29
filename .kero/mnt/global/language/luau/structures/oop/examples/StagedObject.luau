-- structures/oop: StagedObject.luau
--!strict

local Widget = {}
Widget.__index = Widget

type WidgetFields = {
	Root: Frame,
	Connections: { RBXScriptConnection },
}

export type Widget = setmetatable<WidgetFields, typeof(Widget)>

function Widget.new(): Widget
	local self = setmetatable({}, Widget) :: Widget

	self.Root = Instance.new("Frame")
	self.Connections = {}

	return self
end

function Widget.Destroy(self: Widget)
	for _, connection in self.Connections do
		connection:Disconnect()
	end

	self.Root:Destroy()
end

return Widget
