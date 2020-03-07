local _M = {}

function _M.run()
    local headers = { ["Content-type"] = "text/html", ["Custom"] = "test lua" }

    local function hello_text()
        coroutine.yield("<html><body>")
        coroutine.yield("<p>Hello Wsapi!</p>")
        coroutine.yield("</body></html>")
    end

    return 200, headers, coroutine.wrap(hello_text)
end

return _M