using System.IO;
using System.Collections.ObjectModel;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Win32;

namespace SteamGridUI;

public partial class MainWindow : Window
{
    private readonly ObservableCollection<AssetItem> _assets = [];
    private readonly SemaphoreSlim _thumbGate = new(6);
    private List<GameItem> _games = [];
    private int _loadToken;
    private int _page;
    private bool _ready;
    private bool _busy;

    public MainWindow()
    {
        InitializeComponent();
        AssetsList.ItemsSource = _assets;
    }

    private string? UserId => (UserBox.SelectedItem as UserItem)?.Id;
    private string Kind => (string)((ComboBoxItem)KindBox.SelectedItem).Tag;
    private GameItem? Game => GamesList.SelectedItem as GameItem;
    private AssetItem? Asset => AssetsList.SelectedItem as AssetItem;

    private void SetStatus(string? text) => Status.Text = text ?? "";

    // ---- startup & library -------------------------------------------------------------

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        try
        {
            var init = await Native.Call("init");
            var users = init.GetProperty("users").EnumerateArray()
                .Select(u => new UserItem(u.GetProperty("id").GetString()!, u.GetProperty("name").GetString()!))
                .ToList();
            UserBox.ItemsSource = users;
            _ready = true;

            if (init.GetProperty("steam_path").ValueKind != JsonValueKind.String)
                SetStatus(init.GetProperty("steam_error").GetString());
            else if (users.Count == 0)
                SetStatus("No Steam accounts found. Sign in to Steam once, then restart this app.");
            else
                UserBox.SelectedIndex = 0; // most recently used account; loads its games

            if (!init.GetProperty("api_key_set").GetBoolean()) AskApiKey();
        }
        catch (Exception ex)
        {
            SetStatus(ex.Message);
        }
    }

    private async void OnUserChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_ready || UserId is not { } user) return;
        try
        {
            SetStatus("Reading Steam library…");
            var list = await Native.Call("list_games", new { user });
            if (user != UserId) return;
            _games = list.EnumerateArray().Select(GameItem.Parse).ToList();
            GamesList.ItemsSource = _games;
            CollectionViewSource.GetDefaultView(_games).Filter = MatchesFilter;
            SetStatus($"{_games.Count(g => g.Installed)} installed games, {_games.Count} known to this account.");
        }
        catch (Exception ex)
        {
            SetStatus(ex.Message);
        }
    }

    private bool MatchesFilter(object item)
    {
        var g = (GameItem)item;
        if (InstalledOnly.IsChecked == true && !g.Installed) return false;
        var q = SearchBox.Text.Trim();
        return q.Length == 0
            || g.Display.Contains(q, StringComparison.OrdinalIgnoreCase)
            || g.AppId.StartsWith(q, StringComparison.Ordinal);
    }

    private void RefreshFilter() => CollectionViewSource.GetDefaultView(_games).Refresh();
    private void OnSearchChanged(object sender, TextChangedEventArgs e) { if (_ready) RefreshFilter(); }
    private void OnInstalledOnlyClick(object sender, RoutedEventArgs e) => RefreshFilter();

    private void OnApiKey(object sender, RoutedEventArgs e) => AskApiKey();

    private void AskApiKey()
    {
        if (new ApiKeyWindow { Owner = this }.ShowDialog() == true)
        {
            SetStatus("API key saved.");
            _ = LoadAssets(append: false);
        }
    }

    // ---- artwork list ------------------------------------------------------------------

    private void OnGameChanged(object sender, SelectionChangedEventArgs e) { if (_ready) _ = LoadAssets(append: false); }
    private void OnKindChanged(object sender, SelectionChangedEventArgs e) { if (_ready) _ = LoadAssets(append: false); }
    private void OnOptionClick(object sender, RoutedEventArgs e) => _ = LoadAssets(append: false);
    private void OnMore(object sender, RoutedEventArgs e) => _ = LoadAssets(append: true);
    private void OnAssetChanged(object sender, SelectionChangedEventArgs e) => UpdateButtons();

    private async Task LoadAssets(bool append)
    {
        MoreBtn.IsEnabled = false;
        if (!append)
        {
            _loadToken++;
            _page = 0;
            _assets.Clear();
        }
        UpdateButtons();
        if (Game is not { } game) return;

        var token = _loadToken;
        var kind = Kind;
        var page = append ? _page + 1 : 0;
        SetStatus($"Loading {game.Display}…");
        try
        {
            var r = await Native.Call("list_assets", new
            {
                appid = game.AppId,
                kind,
                page,
                animated = AnimatedBox.IsChecked == true,
                humor = HumorBox.IsChecked == true,
                nsfw = NsfwBox.IsChecked == true,
                need_name = !game.HasName,
            });
            if (token != _loadToken) return;

            if (r.GetProperty("game_name").GetString() is { Length: > 0 } name) game.Name = name;
            _page = page;

            var items = r.GetProperty("items").EnumerateArray().Select(e => AssetItem.Parse(e, kind)).ToList();
            foreach (var item in items)
            {
                _assets.Add(item);
                _ = LoadThumb(item, token);
            }

            // SteamGridDB's "total" counts assets the filters hide, so only a full page means there is more.
            var limit = r.GetProperty("limit").ValueKind == JsonValueKind.Number ? r.GetProperty("limit").GetInt32() : 50;
            MoreBtn.IsEnabled = items.Count >= limit;

            SetStatus(
                !r.GetProperty("known").GetBoolean() ? $"SteamGridDB has no entry for appid {game.AppId}."
                : _assets.Count == 0 ? $"No matching artwork for {game.Display}. Try other filters."
                : $"{game.Display}: {_assets.Count} results{(MoreBtn.IsEnabled ? ", more available" : "")}.");
        }
        catch (Exception ex)
        {
            if (token == _loadToken) SetStatus(ex.Message);
        }
    }

    private async Task LoadThumb(AssetItem item, int token)
    {
        if (item.NoPreview) return;
        await _thumbGate.WaitAsync();
        try
        {
            if (token != _loadToken) return;
            var r = await Native.Call("thumb", new { url = item.ThumbUrl });
            var path = r.GetProperty("path").GetString()!;
            item.Thumb = await Task.Run(() => Decode(path));
        }
        catch
        {
            item.Placeholder = "preview unavailable";
        }
        finally
        {
            _thumbGate.Release();
        }
    }

    private static ImageSource Decode(string path)
    {
        var image = new BitmapImage();
        image.BeginInit();
        image.CacheOption = BitmapCacheOption.OnLoad; // read fully now; don't keep the file locked
        image.UriSource = new Uri(path);
        image.EndInit();
        image.Freeze();
        return image;
    }

    // ---- actions -----------------------------------------------------------------------

    private void UpdateButtons()
    {
        var slot = Kind != "icon";
        ApplyBtn.IsEnabled = !_busy && slot && Asset is not null && UserId is not null;
        SaveBtn.IsEnabled = !_busy && Asset is not null;
        RevertBtn.IsEnabled = !_busy && slot && Game is { } g && g.Has(Kind);
        ApplyBtn.ToolTip = slot ? null : "Steam has no custom icon slot for its own games. Use Save as… instead.";
    }

    private async Task RunAction(Func<Task> action)
    {
        _busy = true;
        UpdateButtons();
        try
        {
            await action();
        }
        catch (Exception ex)
        {
            SetStatus(ex.Message);
        }
        finally
        {
            _busy = false;
            UpdateButtons();
        }
    }

    private void OnAssetDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if (ApplyBtn.IsEnabled) OnApply(sender, e);
    }

    private async void OnApply(object sender, RoutedEventArgs e)
    {
        if (Game is not { } game || Asset is not { } asset || UserId is not { } user) return;
        var kind = Kind;
        await RunAction(async () =>
        {
            SetStatus("Downloading…");
            await Native.Call("apply", new { user, appid = game.AppId, kind, url = asset.Url });
            game.SetHas(kind, true);
            SetStatus($"Applied to {game.Display}. Restart Steam if the library doesn't pick it up.");
        });
    }

    private async void OnRevert(object sender, RoutedEventArgs e)
    {
        if (Game is not { } game || UserId is not { } user) return;
        var kind = Kind;
        await RunAction(async () =>
        {
            await Native.Call("clear", new { user, appid = game.AppId, kind });
            game.SetHas(kind, false);
            var dir = (await Native.Call("backup_dir")).GetProperty("path").GetString();
            SetStatus($"Removed custom {Kinds.Label(kind)} art for {game.Display}. The old file is in {dir}.");
        });
    }

    private async void OnSaveAs(object sender, RoutedEventArgs e)
    {
        if (Game is not { } game || Asset is not { } asset) return;
        var ext = Path.GetExtension(new Uri(asset.Url).AbsolutePath);
        var safeName = string.Concat(game.Display.Select(c => Path.GetInvalidFileNameChars().Contains(c) ? '_' : c));
        var dialog = new SaveFileDialog
        {
            FileName = $"{safeName} {Kinds.Label(Kind)}{ext}",
            Filter = $"Image (*{ext})|*{ext}|All files|*.*",
        };
        if (dialog.ShowDialog(this) != true) return;
        await RunAction(async () =>
        {
            SetStatus("Downloading…");
            await Native.Call("save_as", new { url = asset.Url, dest = dialog.FileName });
            SetStatus($"Saved {dialog.FileName}");
        });
    }
}
