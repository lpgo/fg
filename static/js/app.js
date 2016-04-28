var app = angular.module('app',['ngRoute','ngAnimate'],function($httpProvider) {
  // Use x-www-form-urlencoded Content-Type
  $httpProvider.defaults.headers.post['Content-Type'] = 'application/x-www-form-urlencoded;charset=utf-8';
 
  /**
   * The workhorse; converts an object to x-www-form-urlencoded serialization.
   * @param {Object} obj
   * @return {String}
   */ 
  var param = function(obj) {
    var query = '', name, value, fullSubName, subName, subValue, innerObj, i;
      
    for(name in obj) {
      value = obj[name];
        
      if(value instanceof Array) {
        for(i=0; i<value.length; ++i) {
          subValue = value[i];
          fullSubName = name + '[' + i + ']';
          innerObj = {};
          innerObj[fullSubName] = subValue;
          query += param(innerObj) + '&';
        }
      }
      else if(value instanceof Object) {
        for(subName in value) {
          subValue = value[subName];
          fullSubName = name + '[' + subName + ']';
          innerObj = {};
          innerObj[fullSubName] = subValue;
          query += param(innerObj) + '&';
        }
      }
      else if(value !== undefined && value !== null)
        query += encodeURIComponent(name) + '=' + encodeURIComponent(value) + '&';
    }
      
    return query.length ? query.substr(0, query.length - 1) : query;
  };
 
  // Override $http service's default transformRequest
  $httpProvider.defaults.transformRequest = [function(data) {
    return angular.isObject(data) && String(data) !== '[object File]' ? param(data) : data;
  }];
});

app.config(function($routeProvider){
  $routeProvider.when("/",{
    templateUrl:"/static/main.html",
    controller:"MainCtrl"
  });
  $routeProvider.when("/buysit",{
    templateUrl:"/static/buysit.html",
    controller:"BuySeatCtrl"
  });
  $routeProvider.when("/confirmation",{
    templateUrl:"/static/confirmation.html",
    controller:"ConfirmationCtrl"
  });
});

app.controller('BuySeatCtrl', ['$scope', function($scope){
	
}]);

app.controller('ListCtrl', ['$scope','$location', function($scope,$location){
	$scope.count = 15;
	$scope.buySeat = function() {
		$location.url("/buysit");
	}
}]);

app.controller('MainCtrl', ['$scope','$http', function($scope,$http){
	$scope.data = [];
       $scope.getTrips = function() {
          $http.post("/getTrips",null).success(function(d){
            $scope.data = d;
          });
       };
       $scope.getTrips();
}]);

app.controller('ConfirmationCtrl', ['$scope','$http','$location', function(%scope,$http,$location){

  $scope.tel = "";
  $scope.code = "";

  $scope.getCode = function() {
    $http.post("/getCode",$scope).success(function(data){
      if(!data.success) {
        alert("get code error");
      }

    });
  };

  $scope.registerPassenger = function() {
    $http.post("/registerPassenger",$scope).success(function(data){
      if(data.success) {
        $location.url(data.redirect);
      } else {
        alert("registerPassenger error");
      }
    });
  };

}]);