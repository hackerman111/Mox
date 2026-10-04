-- Load the output of mox editor setup with loadstring(code)().
_G.Mox = _G.Mox or {}
function _G.Mox.open(args)
  vim.cmd.edit(vim.fn.fnameescape(args[1]))
  local line_count = vim.api.nvim_buf_line_count(0)
  local line = math.max(1, math.min(math.floor(tonumber(args[2]) or 1), line_count))
  local text = vim.api.nvim_buf_get_lines(0, line - 1, line, false)[1] or ''
  local column = math.max(0, math.min(math.floor(tonumber(args[3]) or 1) - 1, #text - 1))
  vim.api.nvim_win_set_cursor(0, { line, column })
  return 1
end
local function register()
  if not vim.env.TMUX_PANE then return end
  if vim.v.servername == '' then vim.fn.serverstart() end
  vim.fn.jobstart({ 'mox', 'editor', 'register', '--pane', vim.env.TMUX_PANE, '--server', vim.v.servername, '--pid', tostring(vim.fn.getpid()) })
end
vim.api.nvim_create_autocmd('VimEnter', { callback = register, once = true })
if vim.v.vim_did_enter == 1 then register() end
for key, direction in pairs({ h = 'L', j = 'D', k = 'U', l = 'R' }) do
  vim.keymap.set('n', '<C-' .. key .. '>', function()
    local before = vim.api.nvim_get_current_win()
    vim.cmd.wincmd(key)
    if before == vim.api.nvim_get_current_win() then
      vim.fn.jobstart({ 'mox', 'navigate', direction, '--from-editor', '--pane', vim.env.TMUX_PANE or '' })
    end
  end, { silent = true })
end
