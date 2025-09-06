package weather.controller;

import weather.model.DayForecast;
import weather.model.WeatherForecastResponse;
import weather.model.WeatherRequest;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.time.LocalDate;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

@RestController
public class WeatherController {

    private Random random = new Random();

    @PostMapping("/weather")
    public WeatherForecastResponse getRandomWeatherForecast(@RequestBody WeatherRequest req) {
        WeatherForecastResponse response = new WeatherForecastResponse();
        response.setLocation(req.getLocation());
        response.setUnits(req.getUnits());

        List<DayForecast> forecasts = new ArrayList<>();
        for (int i = 0; i < req.getDays(); i++) {
            // Generate a random min temperature between 50 and 89 (inclusive)
            int minTemp = random.nextInt(90 - 50) + 50;
            // Generate a max temperature between (minTemp + 5) and (minTemp + 19)
            int maxTemp = random.nextInt(15) + (minTemp + 5);

            double finalMinTemp = minTemp;
            double finalMaxTemp = maxTemp;

            // Convert to Celsius if necessary
            if (req.getUnits().equalsIgnoreCase("celsius") || req.getUnits().equalsIgnoreCase("c")) {
                finalMinTemp = (minTemp - 32) * 5.0 / 9.0;
                finalMaxTemp = (maxTemp - 32) * 5.0 / 9.0;
            }

            DayForecast dayForecast = new DayForecast();
            dayForecast.setDate(LocalDate.now().plusDays(i).toString());
            dayForecast.setMin(finalMinTemp);
            dayForecast.setMax(finalMaxTemp);
            dayForecast.setUnits(req.getUnits());

            forecasts.add(dayForecast);
        }
        response.setDailyForecast(forecasts);
        return response;
    }
}
