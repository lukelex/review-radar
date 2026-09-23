using Microsoft.UI.Xaml;

namespace ReviewRadar.Windows;

public sealed partial class MainWindow : Window
{
    public QueueStore Queue { get; } = new();

    public MainWindow()
    {
        InitializeComponent();
        Root.DataContext = Queue;
    }

    private async void Root_Loaded(object sender, RoutedEventArgs e)
    {
        await Queue.StartAsync();
    }

    private async void Refresh_Click(object sender, RoutedEventArgs e)
    {
        await Queue.RefreshAsync();
    }
}
