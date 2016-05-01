var app = angular.module('app',['ngRoute'],function($httpProvider) {
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
  $routeProvider.when("/buysit/:oid",{
    templateUrl:"/static/buysit.html",
    controller:"BuySeatCtrl"
  });
  $routeProvider.when("/confirmation",{
    templateUrl:"/static/confirmation.html",
    controller:"ConfirmationCtrl"
  });
  $routeProvider.when("/buysitsuccess",{
    templateUrl:"/static/buysitsuccess.html",
    controller:"BuySuccessCtrl"
  });
  $routeProvider.when("/drivermyline",{
    templateUrl:"/static/drivermyline.html",
    controller:"DriverTripCtrl"
  });
  $routeProvider.when("/driverregister/:userType",{
    templateUrl:"/static/driverregister.html",
    controller:"DriverRegisterCtrl"
  });
  $routeProvider.when("/modifymycar",{
    templateUrl:"/static/modifymycar.html",
    controller:"ModifyCarCtrl"
  });
  $routeProvider.when("/mybill",{
    templateUrl:"/static/mybill.html",
    controller:"MyBillCtrl"
  });
  $routeProvider.when("/mycar",{
    templateUrl:"/static/mycar.html",
    controller:"MyCarCtrl"
  });
  $routeProvider.when("/passengermyline",{
    templateUrl:"/static/passengermyline.html",
    controller:"PassengerTripCtrl"
  });
  $routeProvider.when("/postline",{
    templateUrl:"/static/postline.html",
    controller:"PublishTripCtrl"
  });
  $routeProvider.when("/profile",{
    templateUrl:"/static/profile.html",
    controller:"ProfileCtrl"
  });
  $routeProvider.when("/searchdetail",{
    templateUrl:"/static/searchdetail.html",
    controller:"SearchDetailCtrl"
  });

});

app.controller('BuySeatCtrl', ['$scope','$routeParams','$http','$location','$rootScope',function($scope,$routeParams,$http,$location,$rootScope){
  var oid = $routeParams.oid;
  $rootScope.oid = oid;
  $scope.count =1;
  var getTripDetail = function(oid) {
    $http.post("/tripDetail",{oid:oid}).success(function(trip){
      $scope.trip = trip;
    });
  }
  getTripDetail(oid);

  function pay(data) {
    WeixinJSBridge.invoke(
       'getBrandWCPayRequest', 
       {
          "appId" :data.appId,     
           "timeStamp":data.timeStamp,         //时间戳，自1970年以来的秒数     
           "nonceStr":data.nonceStr, //随机串     
           "package":data.package,     
           "signType":"MD5",         //微信签名方式：     
           "paySign":data.paySign //微信签名 
       },
       function(res){     
           if(res.err_msg == "get_brand_wcpay_request:ok" ) {
                $location.url("/buysitsuccess");
           } else if(res.err_msg == "get_brand_wcpay_request:fail" ) {
                alert("支付失败，请重试！");
           }   
       }
    ); 
  }

  $scope.applyTrip = function() {
    $http.post("/applyTrip",{oid:oid}).success(function(data){
      if(data.success) {
        pay(data);
      } else {
        $location.url("/confirmation");
      }
    });
  }

}]);

app.controller('ListCtrl', ['$scope','$location', function($scope,$location){
	$scope.buySeat = function(oid) {
		$location.url("/buysit/"+oid);
	}
}]);

app.controller('MainCtrl', ['$scope','$http','$location', function($scope,$http,$location){

  $scope.data = [];
  var userType = null;

  $http.post("/getTrips",null).success(function(data){
    $scope.data = data;
  });

  $http.post("/getUserInfo",null).success(function(data){
    if(data.login) {
      userType = data.userType;
    } 
  });
 
  $scope.publishTrip = function() {
    if(userType == "Owner") {
      $location.url("/postline");
    } else if (userType == null) {
      window.location.href="https://open.weixin.qq.com/connect/oauth2/authorize?appid=wxe9a0a490a170731d&redirect_uri=http%3A%2F%2Fgeekgogo.cn%2Fpinche%2Findex.html&response_type=code&scope=snsapi_userinfo&state=STATE#wechat_redirect";
    } else {
      $location.url("/driverregister/"+userType);
    }
  };

}]);

app.controller('ConfirmationCtrl', ['$scope','$http','$location','$rootScope', function($scope,$http,$location,$rootScope){

  $scope.tel = "";
  $scope.code = "";
  $scope.submitBtn = true;
  $scope.getCodeBtn = false;

  $scope.getCode = function() {
    $http.post("/getCode",{tel:$scope.tel}).success(function(data){
      if(!data.success) {
        alert("get code error");
      } else {
        $scope.getCodeBtn = true;
        $scope.submitBtn = false; 
      }
    });
  };

  $scope.registerPassenger = function() {
    $http.post("/registerPassenger",{tel:$scope.tel,code:$scope.code}).success(function(data){
      if(data.success) {
        $location.url("/buysit/"+$rootScope.oid);
      } else {
        alert("registerPassenger error");
      }
    });
  };

}]);

app.controller('BuySuccessCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('DriverTripCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('PassengerTripCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('DriverRegisterCtrl', ['$scope','$location','$routeParams','$http', function($scope,$location,$routeParams,$http){
  
  $scope.carType = "1";

  $scope.needAuth = false;
  $scope.submitBtn = true;
  $scope.getCodeBtn = false;
  
  if($routeParams.userType == "Anonymous") {
    $scope.needAuth = true;
  } else {
    $scope.needAuth = false;
    $scope.submitBtn = false;
  }

  $scope.getCode = function() {
    $http.post("/getCode",{tel:$scope.tel}).success(function(data){
      if(!data.success) {
        alert("get code error");
      } else {
        $scope.submitBtn = false;
        $scope.getCodeBtn = true;
      }
    });
  };

  $scope.becomeOwner = function() {
    $http.post("/registerOwner",{tel:$scope.tel,code:$scope.code,carType:$scope.carType,plateNumber:$scope.plateNumber}).success(function(data){
      if(data.success) {
        $location.url("/postline");
      } else {
        alert("auth code error");
      }
    });
  };


}]);

app.controller('ModifyCarCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('MyBillCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('MyCarCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('PublishTripCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('ProfileCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('SearchDetailCtrl', ['$scope','$location', function($scope,$location){

}]);




