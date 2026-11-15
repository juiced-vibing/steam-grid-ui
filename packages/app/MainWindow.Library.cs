using System.Collections;
using System.Windows;
using System.Windows.Data;

namespace SteamGridUI;

public partial class MainWindow
{
    private static readonly IComparer ById = Comparer<GameItem>.Create((a, b) => a.Id.CompareTo(b.Id));

    private List<GameItem> _games = [];

    private async Task LoadGames()
    {
        if (UserBox.SelectedItem is not User user) return;

        SetStatus("Reading Steam library…");
        var games = await Native.Send(new ListGames(user.Id));
        if (!ReferenceEquals(user, UserBox.SelectedItem)) return;

        _games = games.Select(game => new GameItem(game)).ToList();
        GamesList.ItemsSource = _games;
        CollectionViewSource.GetDefaultView(_games).Filter = MatchesFilter;
        ApplySort();

        SetStatus($"{_games.Count(g => g.Source != Source.Owned)} installed games, {_games.Count} known to this account.");
    }

    private bool MatchesFilter(object item)
    {
        var game = (GameItem)item;
        if (InstalledOnly.IsChecked == true && game.Source == Source.Owned) return false;

        var query = SearchBox.Text.Trim();
        return query.Length == 0
            || game.Display.Contains(query, StringComparison.OrdinalIgnoreCase)
            || game.AppId.StartsWith(query, StringComparison.Ordinal);
    }

    // the core lists by name, so only the other order needs a comparer
    private void ApplySort() =>
        ((ListCollectionView)CollectionViewSource.GetDefaultView(_games)).CustomSort = SortBox.SelectedIndex == 1 ? ById : null;

    private void RefreshFilter() => CollectionViewSource.GetDefaultView(_games).Refresh();

    private void OnInstalledOnlyClick(object sender, RoutedEventArgs e) => RefreshFilter();
}
