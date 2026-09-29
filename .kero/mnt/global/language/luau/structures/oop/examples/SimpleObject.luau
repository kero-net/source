-- structures/oop: SimpleObject.luau
--!strict

local Widget = {}
Widget.__index = Widget

type WidgetFields = {
	Root: Frame,
}

export type Widget = setmetatable<WidgetFields, typeof(Widget)>

function Widget.new(root: Frame): Widget
	local fields: WidgetFields = {
		Root = root,
	}

	return setmetatable(fields, Widget)
end

function Widget.Destroy(self: Widget)
	self.Root:Destroy()
end

return Widget
